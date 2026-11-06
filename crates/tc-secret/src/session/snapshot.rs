//! Instantánea del estado de un chat secreto, para guardarlo y recuperarlo tras
//! reiniciar la aplicación (sans-IO: aquí solo se (de)serializa; dónde se guarda lo
//! decide el llamador).
//!
//! **La instantánea contiene material secreto**: la clave vigente (y la anterior
//! durante un re-keying), los exponentes DH pendientes y los textos en claro del
//! registro de enviados (para atender `Resend`) y de la cola de mensajes fuera de
//! orden. Debe guardarse cifrada o en el keyring del sistema, nunca en claro.
//!
//! Formato propio, binario y versionado (little-endian):
//! `"TCS1" ‖ versión u32 ‖ rol u8 ‖ g u32 ‖ p [256] ‖ versión DH i32 ‖ estado u8 ‖ …`.
//! No es un formato de Telegram ni pretende interoperar con otros clientes.
//!
//! Regla para el llamador: guardar la instantánea **antes** de enviar lo que devuelve
//! cada operación. Si se restaura una instantánea anterior a un envío ya hecho se
//! repetirían números de secuencia (el otro extremo abortaría el chat).

use zeroize::Zeroizing;

use super::{Queued, Ready, Rekey, SecretChat, SentEntry, State};
use crate::crypto::Role;
use crate::dh::{DhParams, DhSecret};
use crate::error::{Result, SecretError};
use crate::key::{KEY_LEN, SecretKey};
use crate::tl::TopLevel;
use crate::visual::{KeyVisualization, VISUAL_LEN};

const MAGIC: &[u8; 4] = b"TCS1";
/// Versión del formato de instantánea.
pub const SNAPSHOT_VERSION: u32 = 1;

const ST_REQUESTED: u8 = 0;
const ST_INCOMING: u8 = 1;
const ST_READY: u8 = 2;
const ST_DISCARDED: u8 = 3;

const RK_IDLE: u8 = 0;
const RK_REQUESTED: u8 = 1;
const RK_ACCEPTED: u8 = 2;
const RK_COMMITTED: u8 = 3;

/// Estado serializado de un chat. Sus bytes se borran al soltarse.
pub struct SessionSnapshot {
    bytes: Zeroizing<Vec<u8>>,
    g: u32,
    p: [u8; KEY_LEN],
    dh_version: i32,
}

impl core::fmt::Debug for SessionSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SessionSnapshot").field("len", &self.bytes.len()).finish_non_exhaustive()
    }
}

impl SessionSnapshot {
    /// Bytes para guardar (en almacenamiento protegido).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Lee la cabecera de unos bytes guardados. El resto se valida al restaurar.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut r = Rd::new(data);
        if r.take(4)? != MAGIC {
            return Err(SecretError::Malformed("instantánea: cabecera desconocida"));
        }
        if r.u32()? != SNAPSHOT_VERSION {
            return Err(SecretError::Malformed("instantánea: versión no soportada"));
        }
        let _role = r.u8()?;
        let g = r.u32()?;
        let p = r.arr()?;
        let dh_version = r.i32()?;
        Ok(Self { bytes: Zeroizing::new(data.to_vec()), g, p, dh_version })
    }

    /// `g` de los parámetros DH del chat.
    pub fn dh_g(&self) -> u32 {
        self.g
    }

    /// `p` de los parámetros DH del chat (big-endian).
    pub fn dh_p(&self) -> &[u8; KEY_LEN] {
        &self.p
    }

    /// `version` de `messages.dhConfig` con la que se creó el chat.
    pub fn dh_version(&self) -> i32 {
        self.dh_version
    }
}

// ---- escritura ----

#[derive(Default)]
struct Wr(Zeroizing<Vec<u8>>);

impl Wr {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn raw(&mut self, b: &[u8]) {
        self.0.extend_from_slice(b);
    }
    fn blob(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.raw(b);
    }
}

// ---- lectura ----

struct Rd<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Rd<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(SecretError::Malformed("instantánea: desbordamiento"))?;
        let s = self.data.get(self.pos..end).ok_or(SecretError::Malformed("instantánea truncada"))?;
        self.pos = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(SecretError::Malformed("instantánea: booleano inválido")),
        }
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn i64(&mut self) -> Result<i64> {
        let mut a = [0u8; 8];
        a.copy_from_slice(self.take(8)?);
        Ok(i64::from_le_bytes(a))
    }
    fn arr<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.take(N)?);
        Ok(a)
    }
    fn key(&mut self) -> Result<SecretKey> {
        let mut a = Zeroizing::new([0u8; KEY_LEN]);
        a.copy_from_slice(self.take(KEY_LEN)?);
        Ok(SecretKey::from_bytes(*a))
    }
    fn blob(&mut self) -> Result<Zeroizing<Vec<u8>>> {
        let n = self.u32()? as usize;
        Ok(Zeroizing::new(self.take(n)?.to_vec()))
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(SecretError::Malformed("instantánea: demasiados elementos"));
        }
        Ok(n)
    }
    fn finish(&self) -> Result<()> {
        if self.pos == self.data.len() { Ok(()) } else { Err(SecretError::Malformed("instantánea: bytes sobrantes")) }
    }
}

fn role_byte(r: Role) -> u8 {
    match r {
        Role::Initiator => 0,
        Role::Responder => 1,
    }
}

impl SecretChat {
    /// Serializa el estado completo (claves incluidas). Ver el aviso del módulo.
    pub fn snapshot(&self) -> SessionSnapshot {
        let mut w = Wr::default();
        w.raw(MAGIC);
        w.u32(SNAPSHOT_VERSION);
        w.u8(role_byte(self.role));
        w.u32(self.dh.g());
        w.raw(self.dh.p());
        w.i32(self.dh.version());
        match &self.state {
            State::Requested(secret) => {
                w.u8(ST_REQUESTED);
                w.raw(secret.exponent_bytes().as_ref());
            }
            State::Incoming { g_a } => {
                w.u8(ST_INCOMING);
                w.raw(g_a.as_slice());
            }
            State::Ready(r) => {
                w.u8(ST_READY);
                write_ready(&mut w, r);
            }
            State::Discarded => w.u8(ST_DISCARDED),
        }
        SessionSnapshot { bytes: w.0, g: self.dh.g(), p: *self.dh.p(), dh_version: self.dh.version() }
    }

    /// Reconstruye un chat desde su instantánea. `dh` deben ser los mismos parámetros
    /// (ya validados) con los que se creó: compruébese con [`SessionSnapshot::dh_p`].
    pub fn restore(dh: DhParams, snap: &SessionSnapshot) -> Result<Self> {
        if dh.g() != snap.g || dh.p() != &snap.p {
            return Err(SecretError::InvalidState("parámetros DH distintos de los del chat"));
        }
        let mut r = Rd::new(&snap.bytes);
        r.take(4 + 4)?;
        let role = match r.u8()? {
            0 => Role::Initiator,
            1 => Role::Responder,
            _ => return Err(SecretError::Malformed("instantánea: rol inválido")),
        };
        r.take(4 + KEY_LEN + 4)?;
        let state = match r.u8()? {
            ST_REQUESTED => {
                let raw = Zeroizing::new(r.arr::<KEY_LEN>()?);
                State::Requested(Box::new(DhSecret::from_exponent_bytes(&dh, &raw)))
            }
            ST_INCOMING => {
                let g_a = r.arr::<KEY_LEN>()?;
                dh.check_public(&g_a)?;
                State::Incoming { g_a: Box::new(g_a) }
            }
            ST_READY => State::Ready(Box::new(read_ready(&mut r, &dh)?)),
            ST_DISCARDED => State::Discarded,
            _ => return Err(SecretError::Malformed("instantánea: estado inválido")),
        };
        r.finish()?;
        Ok(Self { role, dh, state })
    }
}

fn write_ready(w: &mut Wr, r: &Ready) {
    w.raw(r.key.as_bytes());
    w.i64(r.key_created_at);
    w.u32(r.key_used);
    w.u32(r.key_sent);
    match &r.previous {
        Some(k) => {
            w.u8(1);
            w.raw(k.as_bytes());
        }
        None => w.u8(0),
    }
    w.u8(u8::from(r.previous_discardable));
    match &r.rekey {
        Rekey::Idle => w.u8(RK_IDLE),
        Rekey::Requested { exchange_id, secret } => {
            w.u8(RK_REQUESTED);
            w.i64(*exchange_id);
            w.raw(secret.exponent_bytes().as_ref());
        }
        Rekey::Accepted { exchange_id, new_key } => {
            w.u8(RK_ACCEPTED);
            w.i64(*exchange_id);
            w.raw(new_key.as_bytes());
        }
        Rekey::Committed => w.u8(RK_COMMITTED),
    }
    w.raw(r.visual.bytes());
    w.u32(r.out_raw);
    w.u32(r.in_raw);
    w.u32(r.peer_in_raw);
    w.i32(r.peer_layer);
    w.i32(r.ttl);
    w.u8(u8::from(r.gap_requested));
    w.u32(r.sent_log.len() as u32);
    for e in &r.sent_log {
        w.u32(e.raw);
        w.i64(e.random_id);
        w.u8(u8::from(e.service));
        w.blob(&e.payload);
    }
    w.u32(r.queue.len() as u32);
    for (raw, q) in &r.queue {
        w.u32(*raw);
        w.u8(u8::from(q.resend_handled));
        let bytes = Zeroizing::new(q.layer.to_bytes());
        w.blob(&bytes);
    }
}

fn read_ready(r: &mut Rd<'_>, dh: &DhParams) -> Result<Ready> {
    let key = r.key()?;
    let key_created_at = r.i64()?;
    let key_used = r.u32()?;
    let key_sent = r.u32()?;
    let previous = if r.bool()? { Some(r.key()?) } else { None };
    let previous_discardable = r.bool()?;
    let rekey = match r.u8()? {
        RK_IDLE => Rekey::Idle,
        RK_REQUESTED => {
            let exchange_id = r.i64()?;
            let raw = Zeroizing::new(r.arr::<KEY_LEN>()?);
            Rekey::Requested { exchange_id, secret: Box::new(DhSecret::from_exponent_bytes(dh, &raw)) }
        }
        RK_ACCEPTED => {
            let exchange_id = r.i64()?;
            Rekey::Accepted { exchange_id, new_key: Box::new(r.key()?) }
        }
        RK_COMMITTED => Rekey::Committed,
        _ => return Err(SecretError::Malformed("instantánea: re-keying inválido")),
    };
    let visual = KeyVisualization::from_bytes(r.arr::<VISUAL_LEN>()?);
    let out_raw = r.u32()?;
    let in_raw = r.u32()?;
    let peer_in_raw = r.u32()?;
    let peer_layer = r.i32()?;
    let ttl = r.i32()?;
    let gap_requested = r.bool()?;
    let mut sent_log = std::collections::VecDeque::new();
    for _ in 0..r.count(super::SENT_LOG_CAPACITY)? {
        let raw = r.u32()?;
        let random_id = r.i64()?;
        let service = r.bool()?;
        let payload = r.blob()?;
        sent_log.push_back(SentEntry { raw, random_id, service, payload });
    }
    let mut queue = std::collections::BTreeMap::new();
    for _ in 0..r.count(super::MAX_QUEUED)? {
        let raw = r.u32()?;
        let resend_handled = r.bool()?;
        let bytes = r.blob()?;
        let TopLevel::Layer(layer) = TopLevel::parse(&bytes)? else {
            return Err(SecretError::Malformed("instantánea: cola con un mensaje sin capa"));
        };
        queue.insert(raw, Queued { layer, resend_handled });
    }
    Ok(Ready {
        key,
        key_created_at,
        key_used,
        key_sent,
        previous,
        previous_discardable,
        rekey,
        visual,
        out_raw,
        in_raw,
        peer_in_raw,
        peer_layer,
        ttl,
        sent_log,
        queue,
        gap_requested,
    })
}
