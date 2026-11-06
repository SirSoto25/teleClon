//! Máquina de estados de un chat secreto (sans-IO).
//!
//! Estados: `Requested` (lo creamos y esperamos `encryptedChat` con `g_b`) →
//! `Ready`; `Incoming` (nos llegó `encryptedChatRequested` con `g_a`) → `Ready`;
//! cualquier estado → `Discarded`.
//!
//! Cada operación devuelve un [`Output`]: lo que la capa de E/S debe enviar
//! ([`Outbound`], en ese orden) y lo que debe mostrar o guardar ([`Event`]).
//! Ningún método hace red, disco ni lee el reloj: la hora (`now`, segundos Unix)
//! y la entropía las aporta el llamador.
//!
//! Números de secuencia (artículo `end-to-end/seq_no`): contadores crudos
//! `out_raw`/`in_raw` desde 0; en el cable `2·raw + x`, con x = 1 para el
//! `out_seq_no` del creador y para el `in_seq_no` del otro, y x = 0 en los demás
//! casos. Huecos → cola de espera + `decryptedMessageActionResend` (uno por hueco);
//! duplicados → se descartan sin mirar el contenido; paridad o valores imposibles
//! → se aborta el chat.

use std::collections::{BTreeMap, VecDeque};

use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::crypto::{self, Role};
use crate::dh::{DhParams, DhSecret};
use crate::entropy::{Entropy, random_i64, random_up_to};
use crate::error::{Result, SecretError};
use crate::key::{KEY_LEN, SecretKey};
use crate::tl::{
    Action, DecryptedMessage, DecryptedMessageLayer, MIN_RANDOM_BYTES, PlainMessage, TopLevel, TypingAction,
};
use crate::visual::KeyVisualization;

/// Capa de la capa secreta que implementa este crate (entidades de la capa 144).
pub const OUR_LAYER: i32 = 144;
/// Capa supuesta del otro extremo hasta que la notifique (la especificación pide 46).
pub const DEFAULT_PEER_LAYER: i32 = 46;
/// Capa mínima con la que se envía: solo se implementa MTProto 2.0 (capa ≥ 73).
pub const MIN_SEND_LAYER: i32 = 73;
/// Re-keying a partir de más de este número de mensajes cifrados + descifrados.
pub const REKEY_AFTER_MESSAGES: u32 = 100;
/// … o tras este tiempo de uso de la clave (una semana).
pub const REKEY_AFTER_SECS: i64 = 7 * 24 * 3600;
/// Mensajes enviados que se guardan (en memoria) para atender `Resend`.
pub const SENT_LOG_CAPACITY: usize = 1024;
/// Mensajes fuera de orden que se aceptan en espera antes de abortar.
pub const MAX_QUEUED: usize = 1024;

/// Estado observable del chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    /// Lo creamos; esperando a que el otro acepte.
    Requested,
    /// Nos lo pidieron; esperando a que el usuario acepte.
    Incoming,
    /// Clave establecida.
    Ready,
    /// Cerrado.
    Discarded,
}

/// Algo que la capa de red debe enviar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outbound {
    /// `messages.requestEncryption` con este `g_a`.
    RequestEncryption {
        /// `g^a mod p`.
        g_a: [u8; KEY_LEN],
    },
    /// `messages.acceptEncryption`.
    AcceptEncryption {
        /// `g^b mod p`.
        g_b: [u8; KEY_LEN],
        /// Huella de la clave calculada.
        key_fingerprint: i64,
    },
    /// `messages.discardEncryption`.
    DiscardEncryption,
    /// `messages.sendEncrypted` (`service = false`) o `messages.sendEncryptedService`.
    /// Deben salir en orden (cadena `invokeAfterMsgs`).
    SendEncrypted {
        /// `random_id` del mensaje.
        random_id: i64,
        /// Bytes cifrados (`data`).
        data: Vec<u8>,
        /// ¿Mensaje de servicio?
        service: bool,
    },
}

/// Motivo del cierre de un chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscardReason {
    /// Lo cerró el usuario local.
    Local,
    /// Lo cerró el otro extremo o el servidor (`encryptedChatDiscarded`).
    Remote,
    /// La huella de `acceptEncryption` no coincide con la clave calculada.
    FingerprintMismatch,
    /// `g_a`/`g_b` fuera de rango.
    InvalidPublicValue,
    /// Números de secuencia imposibles (paridad, retroceso, futuro).
    SeqViolation,
    /// Nos piden reenviar mensajes que ya no tenemos.
    ResendUnavailable,
    /// Demasiados mensajes fuera de orden.
    TooManyQueued,
    /// `CommitKey` con huella distinta a la aceptada.
    RekeyFailure,
}

/// Algo que la app debe reflejar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Clave establecida.
    Ready {
        /// `key_fingerprint` de la clave.
        fingerprint: i64,
    },
    /// Mensaje de texto recibido (en orden de secuencia).
    Message(PlainMessage),
    /// El otro leyó estos mensajes nuestros: arrancar su temporizador.
    Read(Vec<i64>),
    /// Borrar estos mensajes.
    Deleted(Vec<i64>),
    /// El otro hizo captura de estos mensajes.
    Screenshot(Vec<i64>),
    /// Vaciar historial.
    HistoryFlushed,
    /// Nuevo TTL del chat (segundos).
    TtlChanged(i32),
    /// Escribiendo… / cancelar.
    Typing(TypingAction),
    /// El otro soporta esta capa.
    PeerLayer(i32),
    /// El otro usa una capa más nueva que la nuestra: avisar de que actualice.
    PeerLayerNewer(i32),
    /// Re-keying completado: nueva clave en uso.
    KeyChanged {
        /// Huella de la nueva clave.
        fingerprint: i64,
    },
    /// Re-keying abortado (sin efecto sobre la clave vigente).
    RekeyAborted,
    /// Mensaje repetido, descartado.
    Duplicate {
        /// `out_seq_no` recibido.
        out_seq_no: i32,
    },
    /// Mensaje adelantado, en espera de que se llene el hueco.
    Queued {
        /// `out_seq_no` recibido.
        out_seq_no: i32,
    },
    /// El chat se cerró.
    Discarded(DiscardReason),
}

/// Resultado de una operación.
#[derive(Debug, Default)]
pub struct Output {
    /// A enviar, en orden.
    pub outbound: Vec<Outbound>,
    /// A reflejar.
    pub events: Vec<Event>,
}

struct SentEntry {
    raw: u32,
    random_id: i64,
    service: bool,
    payload: Zeroizing<Vec<u8>>,
}

struct Queued {
    layer: DecryptedMessageLayer,
    resend_handled: bool,
}

enum Rekey {
    Idle,
    /// Enviamos `RequestKey`.
    Requested { exchange_id: i64, secret: Box<DhSecret> },
    /// Enviamos `AcceptKey`; esperamos `CommitKey` (o un mensaje con la nueva clave).
    Accepted { exchange_id: i64, new_key: Box<SecretKey> },
    /// Enviamos `CommitKey`; esperamos un mensaje cifrado con la nueva clave.
    Committed,
}

enum Fail {
    Err(SecretError),
    Abort(DiscardReason),
}

impl From<SecretError> for Fail {
    fn from(e: SecretError) -> Self {
        Fail::Err(e)
    }
}

type Step = core::result::Result<(), Fail>;

struct Ready {
    key: SecretKey,
    key_created_at: i64,
    key_used: u32,
    key_sent: u32,
    previous: Option<SecretKey>,
    previous_discardable: bool,
    rekey: Rekey,
    visual: KeyVisualization,
    out_raw: u32,
    in_raw: u32,
    peer_in_raw: u32,
    peer_layer: i32,
    ttl: i32,
    sent_log: VecDeque<SentEntry>,
    queue: BTreeMap<u32, Queued>,
    gap_requested: bool,
}

enum State {
    Requested(Box<DhSecret>),
    Incoming { g_a: Box<[u8; KEY_LEN]> },
    Ready(Box<Ready>),
    Discarded,
}

/// Un chat secreto, de un extremo.
pub struct SecretChat {
    role: Role,
    dh: DhParams,
    state: State,
}

impl core::fmt::Debug for SecretChat {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SecretChat").field("role", &self.role).field("state", &self.state_kind()).finish()
    }
}

/// x del `out_seq_no` que ENVÍA este rol.
fn x_out(role: Role) -> i32 {
    match role {
        Role::Initiator => 1,
        Role::Responder => 0,
    }
}

/// x del `in_seq_no` que ENVÍA este rol.
fn x_in(role: Role) -> i32 {
    1 - x_out(role)
}

fn wire(raw: u32, x: i32) -> core::result::Result<i32, Fail> {
    i32::try_from(raw)
        .ok()
        .and_then(|r| r.checked_mul(2))
        .and_then(|r| r.checked_add(x))
        .ok_or(Fail::Err(SecretError::InvalidState("contador de secuencia agotado")))
}

fn random_bytes(entropy: &mut dyn Entropy) -> Result<Vec<u8>> {
    let n = MIN_RANDOM_BYTES + 1 + random_up_to(entropy, 16)? as usize;
    let mut v = vec![0u8; n];
    entropy.fill(&mut v)?;
    Ok(v)
}

fn nonzero_random_id(entropy: &mut dyn Entropy) -> Result<i64> {
    loop {
        let id = random_i64(entropy)?;
        if id != 0 {
            return Ok(id);
        }
    }
}

impl Ready {
    fn new(key: SecretKey, now: i64) -> Self {
        let visual = KeyVisualization::new(&key, &key);
        Self {
            key,
            key_created_at: now,
            key_used: 0,
            key_sent: 0,
            previous: None,
            previous_discardable: false,
            rekey: Rekey::Idle,
            visual,
            out_raw: 0,
            in_raw: 0,
            peer_in_raw: 0,
            peer_layer: DEFAULT_PEER_LAYER,
            ttl: 0,
            sent_log: VecDeque::new(),
            queue: BTreeMap::new(),
            gap_requested: false,
        }
    }

    fn send_layer_message(
        &mut self,
        role: Role,
        message: DecryptedMessage,
        entropy: &mut dyn Entropy,
    ) -> core::result::Result<Outbound, Fail> {
        let service = matches!(message, DecryptedMessage::Service { .. });
        let random_id = message.random_id();
        let layer = DecryptedMessageLayer {
            random_bytes: random_bytes(entropy)?,
            layer: OUR_LAYER.min(self.peer_layer).max(MIN_SEND_LAYER),
            in_seq_no: wire(self.in_raw, x_in(role))?,
            out_seq_no: wire(self.out_raw, x_out(role))?,
            message,
        };
        let payload = Zeroizing::new(layer.to_bytes());
        let data = crypto::encrypt(&self.key, role, &payload, entropy)?;
        self.sent_log.push_back(SentEntry { raw: self.out_raw, random_id, service, payload });
        if self.sent_log.len() > SENT_LOG_CAPACITY {
            self.sent_log.pop_front();
        }
        self.out_raw += 1;
        self.key_used = self.key_used.saturating_add(1);
        self.key_sent = self.key_sent.saturating_add(1);
        Ok(Outbound::SendEncrypted { random_id, data, service })
    }

    fn send_service(&mut self, role: Role, action: Action, entropy: &mut dyn Entropy, out: &mut Output) -> Step {
        let random_id = nonzero_random_id(entropy)?;
        let o = self.send_layer_message(role, DecryptedMessage::Service { random_id, action }, entropy)?;
        out.outbound.push(o);
        Ok(())
    }

    /// `NotifyLayer` dentro del `decryptedMessageService` obsoleto de capa 8 (sin secuencia).
    fn send_notify_layer(&mut self, role: Role, entropy: &mut dyn Entropy, out: &mut Output) -> Result<()> {
        let random_id = nonzero_random_id(entropy)?;
        let msg = TopLevel::LegacyService {
            random_id,
            random_bytes: random_bytes(entropy)?,
            action: Action::NotifyLayer { layer: OUR_LAYER },
        };
        let payload = Zeroizing::new(msg.to_bytes());
        let data = crypto::encrypt(&self.key, role, &payload, entropy)?;
        self.key_used = self.key_used.saturating_add(1);
        self.key_sent = self.key_sent.saturating_add(1);
        out.outbound.push(Outbound::SendEncrypted { random_id, data, service: true });
        Ok(())
    }

    fn update_peer_layer(&mut self, layer: i32, out: &mut Output) {
        if layer >= DEFAULT_PEER_LAYER && layer > self.peer_layer {
            self.peer_layer = layer;
            out.events.push(Event::PeerLayer(layer));
        }
        if layer > OUR_LAYER {
            out.events.push(Event::PeerLayerNewer(layer));
        }
    }

    fn switch_key(&mut self, new_key: SecretKey, discardable: bool, now: i64, out: &mut Output) {
        let old = core::mem::replace(&mut self.key, new_key);
        self.previous = Some(old);
        self.previous_discardable = discardable;
        self.key_created_at = now;
        self.key_used = 0;
        self.key_sent = 0;
        out.events.push(Event::KeyChanged { fingerprint: self.key.fingerprint() });
    }

    /// Lado B: instalar la clave aceptada (por `CommitKey` o por recibir un mensaje con ella).
    fn install_pending(&mut self, role: Role, now: i64, entropy: &mut dyn Entropy, out: &mut Output) -> Step {
        if let Rekey::Accepted { new_key, .. } = core::mem::replace(&mut self.rekey, Rekey::Idle) {
            self.switch_key(*new_key, true, now, out);
            // Para que A reciba algo cifrado con la nueva clave y pueda descartar la vieja.
            self.send_service(role, Action::Noop, entropy, out)?;
        }
        Ok(())
    }

    fn maybe_drop_previous(&mut self) {
        if self.previous_discardable && self.queue.is_empty() {
            self.previous = None;
            self.previous_discardable = false;
        }
    }

    fn request_gap(&mut self, role: Role, next_raw: u32, entropy: &mut dyn Entropy, out: &mut Output) -> Step {
        let peer_x = x_out(role.peer());
        let start = wire(self.in_raw, peer_x)?;
        let end = wire(next_raw, peer_x)? - 2;
        self.gap_requested = true;
        self.send_service(role, Action::Resend { start_seq_no: start, end_seq_no: end }, entropy, out)
    }

    fn handle_resend(&mut self, role: Role, start: i32, end: i32, entropy: &mut dyn Entropy, out: &mut Output) -> Step {
        let x = x_out(role);
        if start < 0 || end < start || start % 2 != x || end % 2 != x {
            return Err(Fail::Abort(DiscardReason::SeqViolation));
        }
        let (raw_start, raw_end) = ((start / 2) as u32, (end / 2) as u32);
        if raw_end >= self.out_raw {
            return Err(Fail::Abort(DiscardReason::SeqViolation));
        }
        if (raw_end - raw_start) as usize >= SENT_LOG_CAPACITY {
            return Err(Fail::Abort(DiscardReason::ResendUnavailable));
        }
        for raw in raw_start..=raw_end {
            let Some(entry) = self.sent_log.iter().find(|e| e.raw == raw) else {
                return Err(Fail::Abort(DiscardReason::ResendUnavailable));
            };
            // Se re-cifra con la clave vigente (la anterior puede estar ya descartada).
            let data = crypto::encrypt(&self.key, role, &entry.payload, entropy)?;
            out.outbound.push(Outbound::SendEncrypted { random_id: entry.random_id, data, service: entry.service });
            self.key_used = self.key_used.saturating_add(1);
            self.key_sent = self.key_sent.saturating_add(1);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn on_request_key(
        &mut self,
        role: Role,
        dh: &DhParams,
        exchange_id: i64,
        g_a: &[u8],
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        match &self.rekey {
            Rekey::Idle => {}
            Rekey::Requested { exchange_id: ours, .. } => {
                // Peticiones cruzadas: se aborta la instancia de exchange_id menor
                // (comparación como long con signo); si son iguales, ambas.
                let ours = *ours;
                if ours > exchange_id {
                    return Ok(());
                }
                self.rekey = Rekey::Idle;
                if ours == exchange_id {
                    out.events.push(Event::RekeyAborted);
                    return Ok(());
                }
            }
            Rekey::Accepted { .. } | Rekey::Committed => {
                return self.send_service(role, Action::AbortKey { exchange_id }, entropy, out);
            }
        }
        if dh.check_public(g_a).is_err() {
            out.events.push(Event::RekeyAborted);
            return self.send_service(role, Action::AbortKey { exchange_id }, entropy, out);
        }
        let secret = DhSecret::generate(dh, entropy, &[])?;
        let new_key = secret.shared_key(dh, g_a)?;
        let action = Action::AcceptKey { exchange_id, g_b: secret.public().to_vec(), key_fingerprint: new_key.fingerprint() };
        self.send_service(role, action, entropy, out)?;
        self.rekey = Rekey::Accepted { exchange_id, new_key: Box::new(new_key) };
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn on_accept_key(
        &mut self,
        role: Role,
        dh: &DhParams,
        exchange_id: i64,
        g_b: &[u8],
        fingerprint: i64,
        now: i64,
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        let matches = matches!(&self.rekey, Rekey::Requested { exchange_id: e, .. } if *e == exchange_id);
        if !matches {
            return Ok(()); // instancia desconocida o ya abortada
        }
        let Rekey::Requested { secret, .. } = core::mem::replace(&mut self.rekey, Rekey::Idle) else {
            return Ok(());
        };
        let new_key = match secret.shared_key(dh, g_b) {
            Ok(k) if bool::from(k.fingerprint().to_le_bytes().ct_eq(&fingerprint.to_le_bytes())) => k,
            _ => {
                out.events.push(Event::RekeyAborted);
                return self.send_service(role, Action::AbortKey { exchange_id }, entropy, out);
            }
        };
        // CommitKey va cifrado aún con la clave vieja; todo lo posterior, con la nueva.
        self.send_service(role, Action::CommitKey { exchange_id, key_fingerprint: fingerprint }, entropy, out)?;
        self.switch_key(new_key, false, now, out);
        self.rekey = Rekey::Committed;
        Ok(())
    }

    fn on_commit_key(
        &mut self,
        role: Role,
        exchange_id: i64,
        fingerprint: i64,
        now: i64,
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        match &self.rekey {
            Rekey::Accepted { exchange_id: e, new_key } if *e == exchange_id => {
                if new_key.fingerprint() != fingerprint {
                    return Err(Fail::Abort(DiscardReason::RekeyFailure));
                }
                self.install_pending(role, now, entropy, out)
            }
            // Ya cambiamos al recibir un mensaje con la nueva clave: nada que hacer.
            _ => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dispatch(
        &mut self,
        role: Role,
        dh: &DhParams,
        message: DecryptedMessage,
        resend_handled: bool,
        now: i64,
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        match message {
            DecryptedMessage::Message(m) => out.events.push(Event::Message(m)),
            DecryptedMessage::Service { action, .. } => match action {
                Action::SetMessageTtl { ttl_seconds } => {
                    self.ttl = ttl_seconds.max(0);
                    out.events.push(Event::TtlChanged(self.ttl));
                }
                Action::ReadMessages { random_ids } => out.events.push(Event::Read(random_ids)),
                Action::DeleteMessages { random_ids } => out.events.push(Event::Deleted(random_ids)),
                Action::ScreenshotMessages { random_ids } => out.events.push(Event::Screenshot(random_ids)),
                Action::FlushHistory => out.events.push(Event::HistoryFlushed),
                Action::Resend { start_seq_no, end_seq_no } => {
                    if !resend_handled {
                        self.handle_resend(role, start_seq_no, end_seq_no, entropy, out)?;
                    }
                }
                Action::NotifyLayer { layer } => self.update_peer_layer(layer, out),
                Action::Typing(t) => out.events.push(Event::Typing(t)),
                Action::RequestKey { exchange_id, g_a } => {
                    self.on_request_key(role, dh, exchange_id, &g_a, entropy, out)?;
                }
                Action::AcceptKey { exchange_id, g_b, key_fingerprint } => {
                    self.on_accept_key(role, dh, exchange_id, &g_b, key_fingerprint, now, entropy, out)?;
                }
                Action::AbortKey { exchange_id } => {
                    let ours = match &self.rekey {
                        Rekey::Requested { exchange_id: e, .. } | Rekey::Accepted { exchange_id: e, .. } => {
                            *e == exchange_id
                        }
                        _ => false,
                    };
                    if ours {
                        self.rekey = Rekey::Idle;
                        out.events.push(Event::RekeyAborted);
                    }
                }
                Action::CommitKey { exchange_id, key_fingerprint } => {
                    self.on_commit_key(role, exchange_id, key_fingerprint, now, entropy, out)?;
                }
                Action::Noop => {}
            },
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_in_order(
        &mut self,
        role: Role,
        dh: &DhParams,
        layer: DecryptedMessageLayer,
        resend_handled: bool,
        now: i64,
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        let raw_in = (layer.in_seq_no / 2) as u32;
        if raw_in < self.peer_in_raw {
            return Err(Fail::Abort(DiscardReason::SeqViolation));
        }
        self.peer_in_raw = raw_in;
        self.in_raw += 1;
        self.dispatch(role, dh, layer.message, resend_handled, now, entropy, out)
    }

    fn receive_layer(
        &mut self,
        role: Role,
        dh: &DhParams,
        layer: DecryptedMessageLayer,
        now: i64,
        entropy: &mut dyn Entropy,
        out: &mut Output,
    ) -> Step {
        if layer.random_bytes.len() < MIN_RANDOM_BYTES {
            return Err(Fail::Err(SecretError::Malformed("random_bytes con menos de 15 bytes")));
        }
        if layer.layer < 17 {
            return Err(Fail::Err(SecretError::Malformed("capa < 17 en decryptedMessageLayer")));
        }
        self.update_peer_layer(layer.layer, out);

        let (out_seq, in_seq) = (layer.out_seq_no, layer.in_seq_no);
        if out_seq < 0 || in_seq < 0 || out_seq % 2 != x_out(role.peer()) || in_seq % 2 != x_in(role.peer()) {
            return Err(Fail::Abort(DiscardReason::SeqViolation));
        }
        let raw_out = (out_seq / 2) as u32;
        let raw_in = (in_seq / 2) as u32;
        if raw_in > self.out_raw {
            // El otro dice haber recibido mensajes nuestros que nunca enviamos.
            return Err(Fail::Abort(DiscardReason::SeqViolation));
        }
        if raw_out < self.in_raw {
            out.events.push(Event::Duplicate { out_seq_no: out_seq });
            return Ok(());
        }
        // Resend se atiende al llegar, aunque venga adelantado (evita huecos cruzados).
        let mut resend_handled = false;
        if let DecryptedMessage::Service { action: Action::Resend { start_seq_no, end_seq_no }, .. } = &layer.message {
            if raw_out > self.in_raw && self.queue.contains_key(&raw_out) {
                out.events.push(Event::Duplicate { out_seq_no: out_seq });
                return Ok(());
            }
            self.handle_resend(role, *start_seq_no, *end_seq_no, entropy, out)?;
            resend_handled = true;
        }
        if raw_out > self.in_raw {
            if self.queue.contains_key(&raw_out) {
                out.events.push(Event::Duplicate { out_seq_no: out_seq });
                return Ok(());
            }
            if self.queue.len() >= MAX_QUEUED {
                return Err(Fail::Abort(DiscardReason::TooManyQueued));
            }
            self.queue.insert(raw_out, Queued { layer, resend_handled });
            out.events.push(Event::Queued { out_seq_no: out_seq });
            if !self.gap_requested {
                self.request_gap(role, raw_out, entropy, out)?;
            }
            return Ok(());
        }
        self.process_in_order(role, dh, layer, resend_handled, now, entropy, out)?;
        while let Some(q) = self.queue.remove(&self.in_raw) {
            self.process_in_order(role, dh, q.layer, q.resend_handled, now, entropy, out)?;
        }
        self.gap_requested = false;
        if let Some((&next, _)) = self.queue.first_key_value() {
            self.request_gap(role, next, entropy, out)?;
        }
        self.maybe_drop_previous();
        Ok(())
    }
}

fn ready_of(state: &mut State) -> Result<&mut Ready> {
    match state {
        State::Ready(r) => Ok(r),
        State::Discarded => Err(SecretError::InvalidState("chat descartado")),
        _ => Err(SecretError::InvalidState("la clave aún no está establecida")),
    }
}

impl SecretChat {
    /// Crea un chat secreto como iniciador. Devuelve el chat y el `g_a` que hay que
    /// enviar con `messages.requestEncryption`.
    pub fn request(dh: DhParams, entropy: &mut dyn Entropy, server_random: &[u8]) -> Result<(Self, Output)> {
        let secret = DhSecret::generate(&dh, entropy, server_random)?;
        let out = Output { outbound: vec![Outbound::RequestEncryption { g_a: *secret.public() }], events: vec![] };
        Ok((Self { role: Role::Initiator, dh, state: State::Requested(Box::new(secret)) }, out))
    }

    /// Registra una solicitud entrante (`encryptedChatRequested`). Valida `g_a` ya:
    /// si es inválido devuelve error y la capa de red debe descartar el chat.
    pub fn from_request(dh: DhParams, g_a: &[u8]) -> Result<Self> {
        dh.check_public(g_a)?;
        let g_a = crate::dh::left_pad(g_a)?;
        Ok(Self { role: Role::Responder, dh, state: State::Incoming { g_a: Box::new(g_a) } })
    }

    /// El usuario acepta la solicitud: calcula la clave y devuelve `acceptEncryption`
    /// seguido de la notificación de capa.
    pub fn accept(&mut self, entropy: &mut dyn Entropy, server_random: &[u8], now: i64) -> Result<Output> {
        let State::Incoming { g_a } = &self.state else {
            return Err(SecretError::InvalidState("no hay solicitud pendiente"));
        };
        let secret = DhSecret::generate(&self.dh, entropy, server_random)?;
        let key = secret.shared_key(&self.dh, g_a.as_slice())?;
        let fingerprint = key.fingerprint();
        let mut out = Output::default();
        out.outbound.push(Outbound::AcceptEncryption { g_b: *secret.public(), key_fingerprint: fingerprint });
        let mut ready = Box::new(Ready::new(key, now));
        ready.send_notify_layer(self.role, entropy, &mut out)?;
        out.events.push(Event::Ready { fingerprint });
        self.state = State::Ready(ready);
        Ok(out)
    }

    /// Iniciador: el otro aceptó (`encryptedChat` con `g_a_or_b` y `key_fingerprint`).
    /// Si `g_b` es inválido o la huella no coincide, el chat se descarta (no es un `Err`).
    pub fn on_accepted(&mut self, g_b: &[u8], key_fingerprint: i64, entropy: &mut dyn Entropy, now: i64) -> Result<Output> {
        let State::Requested(secret) = &self.state else {
            return Err(SecretError::InvalidState("no esperábamos aceptación"));
        };
        let mut out = Output::default();
        let key = match secret.shared_key(&self.dh, g_b) {
            Ok(k) => k,
            Err(SecretError::InvalidPublicValue) => {
                self.discard_with(DiscardReason::InvalidPublicValue, &mut out);
                return Ok(out);
            }
            Err(e) => return Err(e),
        };
        if !bool::from(key.fingerprint().to_le_bytes().ct_eq(&key_fingerprint.to_le_bytes())) {
            self.discard_with(DiscardReason::FingerprintMismatch, &mut out);
            return Ok(out);
        }
        let mut ready = Box::new(Ready::new(key, now));
        ready.send_notify_layer(self.role, entropy, &mut out)?;
        out.events.push(Event::Ready { fingerprint: key_fingerprint });
        self.state = State::Ready(ready);
        Ok(out)
    }

    fn ready_mut(&mut self) -> Result<&mut Ready> {
        ready_of(&mut self.state)
    }

    fn ready(&self) -> Option<&Ready> {
        match &self.state {
            State::Ready(r) => Some(r),
            _ => None,
        }
    }

    fn finish(&mut self, step: Step, mut out: Output) -> Result<Output> {
        match step {
            Ok(()) => Ok(out),
            Err(Fail::Abort(reason)) => {
                self.discard_with(reason, &mut out);
                Ok(out)
            }
            Err(Fail::Err(e)) => Err(e),
        }
    }

    fn discard_with(&mut self, reason: DiscardReason, out: &mut Output) {
        if matches!(self.state, State::Discarded) {
            return;
        }
        // Al sustituir el estado se sueltan (y borran) clave, exponentes y registro.
        self.state = State::Discarded;
        if reason != DiscardReason::Remote {
            out.outbound.push(Outbound::DiscardEncryption);
        }
        out.events.push(Event::Discarded(reason));
    }

    /// Envía un mensaje de texto con el TTL vigente del chat. Devuelve su `random_id`.
    pub fn send_text(&mut self, text: &str, entropy: &mut dyn Entropy) -> Result<(i64, Output)> {
        let ttl = self.ready_mut()?.ttl;
        self.send_message(PlainMessage { message: text.to_owned(), ttl, ..PlainMessage::default() }, entropy)
    }

    /// Envía un mensaje. Si `random_id == 0` se genera uno.
    pub fn send_message(&mut self, mut message: PlainMessage, entropy: &mut dyn Entropy) -> Result<(i64, Output)> {
        let role = self.role;
        let ready = self.ready_mut()?;
        if message.random_id == 0 {
            message.random_id = nonzero_random_id(entropy)?;
        }
        let id = message.random_id;
        let mut out = Output::default();
        let step = ready.send_layer_message(role, DecryptedMessage::Message(message), entropy).map(|o| out.outbound.push(o));
        Ok((id, self.finish(step, out)?))
    }

    /// Envía una acción de servicio de usuario (TTL, leídos, borrar, captura,
    /// vaciar, escribiendo, noop). Las de re-keying y `Resend` son internas.
    pub fn send_action(&mut self, action: Action, entropy: &mut dyn Entropy) -> Result<Output> {
        if matches!(
            action,
            Action::Resend { .. }
                | Action::RequestKey { .. }
                | Action::AcceptKey { .. }
                | Action::AbortKey { .. }
                | Action::CommitKey { .. }
                | Action::NotifyLayer { .. }
        ) {
            return Err(SecretError::InvalidState("acción interna del protocolo"));
        }
        let role = self.role;
        let ready = self.ready_mut()?;
        if let Action::SetMessageTtl { ttl_seconds } = action {
            ready.ttl = ttl_seconds.max(0);
        }
        let mut out = Output::default();
        let step = ready.send_service(role, action, entropy, &mut out);
        self.finish(step, out)
    }

    /// Cambia el temporizador de autodestrucción del chat (segundos, 0 = apagado).
    pub fn set_ttl(&mut self, ttl_seconds: i32, entropy: &mut dyn Entropy) -> Result<Output> {
        self.send_action(Action::SetMessageTtl { ttl_seconds }, entropy)
    }

    /// Procesa un mensaje cifrado entrante (`encryptedMessage(Service).bytes`).
    ///
    /// - Error (`Err`): el mensaje se ignora y el estado NO cambia (manipulado,
    ///   clave desconocida, TL inválido, `random_bytes` insuficientes).
    /// - Violación de protocolo (secuencias, reenvío imposible…): el chat se
    ///   aborta y `Output` trae `DiscardEncryption` + `Event::Discarded`.
    pub fn receive(&mut self, data: &[u8], entropy: &mut dyn Entropy, now: i64) -> Result<Output> {
        let role = self.role;
        let dh = &self.dh;
        let ready = ready_of(&mut self.state)?;
        let fp = crypto::peek_fingerprint(data)?;

        #[derive(PartialEq)]
        enum Which {
            Current,
            Pending,
            Previous,
        }
        let which = if fp == ready.key.fingerprint() {
            Which::Current
        } else if matches!(&ready.rekey, Rekey::Accepted { new_key, .. } if new_key.fingerprint() == fp) {
            Which::Pending
        } else if ready.previous.as_ref().is_some_and(|k| k.fingerprint() == fp) {
            Which::Previous
        } else {
            return Err(SecretError::UnknownKeyFingerprint);
        };
        let payload = {
            let key: &SecretKey = match which {
                Which::Current => &ready.key,
                Which::Pending => match &ready.rekey {
                    Rekey::Accepted { new_key, .. } => new_key,
                    _ => unreachable!("comprobado arriba"),
                },
                Which::Previous => ready.previous.as_ref().expect("comprobado arriba"),
            };
            crypto::decrypt(key, role.peer(), data)?
        };
        let parsed = TopLevel::parse(&payload)?;
        if let TopLevel::Layer(l) = &parsed
            && l.random_bytes.len() < MIN_RANDOM_BYTES
        {
            return Err(SecretError::Malformed("random_bytes con menos de 15 bytes"));
        }

        let mut out = Output::default();
        let step = (|| -> Step {
            match which {
                Which::Current => {
                    ready.key_used = ready.key_used.saturating_add(1);
                    if matches!(ready.rekey, Rekey::Committed) {
                        // A: el otro ya cifra con la nueva clave → la vieja sobra.
                        ready.rekey = Rekey::Idle;
                        ready.previous_discardable = true;
                    }
                }
                Which::Pending => {
                    ready.install_pending(role, now, entropy, &mut out)?;
                    ready.key_used = ready.key_used.saturating_add(1);
                }
                Which::Previous => {}
            }
            match parsed {
                TopLevel::LegacyService { random_bytes, action, .. } => {
                    if random_bytes.len() < MIN_RANDOM_BYTES {
                        return Err(Fail::Err(SecretError::Malformed("random_bytes con menos de 15 bytes")));
                    }
                    match action {
                        Action::NotifyLayer { layer } => ready.update_peer_layer(layer, &mut out),
                        _ => return Err(Fail::Err(SecretError::Tl("servicio de capa 8 no soportado"))),
                    }
                }
                TopLevel::Layer(l) => ready.receive_layer(role, dh, l, now, entropy, &mut out)?,
            }
            ready.maybe_drop_previous();
            Ok(())
        })();
        self.finish(step, out)
    }

    /// ¿Toca re-keying? (más de 100 mensajes con esta clave o más de una semana,
    /// habiendo cifrado al menos uno, y sin otra instancia en curso).
    pub fn should_rekey(&self, now: i64) -> bool {
        self.ready().is_some_and(|r| {
            matches!(r.rekey, Rekey::Idle)
                && r.previous.is_none()
                && r.key_sent >= 1
                && (r.key_used > REKEY_AFTER_MESSAGES || now - r.key_created_at > REKEY_AFTER_SECS)
        })
    }

    /// Inicia el re-keying (envía `RequestKey`). Reutiliza `p` y `g` del chat.
    pub fn start_rekey(&mut self, entropy: &mut dyn Entropy) -> Result<Output> {
        let role = self.role;
        let dh = &self.dh;
        let ready = ready_of(&mut self.state)?;
        if !matches!(ready.rekey, Rekey::Idle) {
            return Err(SecretError::InvalidState("ya hay un re-keying en curso"));
        }
        let exchange_id = random_i64(entropy)?;
        let secret = DhSecret::generate(dh, entropy, &[])?;
        let mut out = Output::default();
        let action = Action::RequestKey { exchange_id, g_a: secret.public().to_vec() };
        let step = ready.send_service(role, action, entropy, &mut out);
        if step.is_ok() {
            ready.rekey = Rekey::Requested { exchange_id, secret: Box::new(secret) };
        }
        self.finish(step, out)
    }

    /// El usuario cierra el chat.
    pub fn discard(&mut self) -> Output {
        let mut out = Output::default();
        self.discard_with(DiscardReason::Local, &mut out);
        out
    }

    /// Llegó `encryptedChatDiscarded`.
    pub fn on_remote_discarded(&mut self) -> Output {
        let mut out = Output::default();
        self.discard_with(DiscardReason::Remote, &mut out);
        out
    }

    // ---- consultas ----

    /// Rol local.
    pub fn role(&self) -> Role {
        self.role
    }

    /// Estado.
    pub fn state_kind(&self) -> StateKind {
        match self.state {
            State::Requested(_) => StateKind::Requested,
            State::Incoming { .. } => StateKind::Incoming,
            State::Ready(_) => StateKind::Ready,
            State::Discarded => StateKind::Discarded,
        }
    }

    /// Huella de la clave vigente.
    pub fn key_fingerprint(&self) -> Option<i64> {
        self.ready().map(|r| r.key.fingerprint())
    }

    /// Clave vigente, para guardarla en el keyring. No registrar nunca.
    pub fn current_key(&self) -> Option<&SecretKey> {
        self.ready().map(|r| &r.key)
    }

    /// Visualización de la clave (fija aunque haya re-keying).
    pub fn key_visualization(&self) -> Option<&KeyVisualization> {
        self.ready().map(|r| &r.visual)
    }

    /// TTL vigente del chat.
    pub fn ttl(&self) -> i32 {
        self.ready().map_or(0, |r| r.ttl)
    }

    /// Capa conocida del otro extremo.
    pub fn peer_layer(&self) -> i32 {
        self.ready().map_or(DEFAULT_PEER_LAYER, |r| r.peer_layer)
    }

    /// Contadores crudos `(out_raw, in_raw)`.
    pub fn seq_counters(&self) -> Option<(u32, u32)> {
        self.ready().map(|r| (r.out_raw, r.in_raw))
    }

    /// Mensajes en espera por huecos.
    pub fn queued(&self) -> usize {
        self.ready().map_or(0, |r| r.queue.len())
    }

    /// ¿Sigue guardada la clave anterior (transición de re-keying)?
    pub fn has_previous_key(&self) -> bool {
        self.ready().is_some_and(|r| r.previous.is_some())
    }

    /// ¿Hay un re-keying en curso?
    pub fn rekey_in_progress(&self) -> bool {
        self.ready().is_some_and(|r| !matches!(r.rekey, Rekey::Idle))
    }
}
