//! Serializador TL propio del subconjunto de la capa de chats secretos que usa
//! teleClon. Constructores tomados del esquema público `end-to-end` (capa 144 para
//! entidades; `decryptedMessage#91cc4674` de la capa 73).
//!
//! Medios: solo `decryptedMessageMediaEmpty`. Cualquier otro medio, entidad o
//! acción no listada se rechaza con [`SecretError::UnsupportedConstructor`]
//! (TL no permite saltar un constructor desconocido sin conocer su forma).

use crate::error::{Result, SecretError};

/// `decryptedMessageLayer#1be31789`.
pub const ID_LAYER: u32 = 0x1be3_1789;
/// `decryptedMessage#91cc4674` (capa 73).
pub const ID_MESSAGE: u32 = 0x91cc_4674;
/// `decryptedMessageService#73164160` (capa 17).
pub const ID_SERVICE: u32 = 0x7316_4160;
/// `decryptedMessageService#aa48327d` (capa 8, obsoleto; solo para `NotifyLayer`).
pub const ID_SERVICE_L8: u32 = 0xaa48_327d;
/// `decryptedMessageMediaEmpty#89f5c4a`.
pub const ID_MEDIA_EMPTY: u32 = 0x089f_5c4a;
const ID_VECTOR: u32 = 0x1cb5_c415;

const ID_SET_TTL: u32 = 0xa173_3aec;
const ID_READ: u32 = 0x0c4f_40be;
const ID_DELETE: u32 = 0x6561_4304;
const ID_SCREENSHOT: u32 = 0x8ac1_f475;
const ID_FLUSH: u32 = 0x6719_e45c;
const ID_RESEND: u32 = 0x5111_10b0;
const ID_NOTIFY_LAYER: u32 = 0xf304_8883;
const ID_TYPING: u32 = 0xccb2_7641;
const ID_REQUEST_KEY: u32 = 0xf3c9_611b;
const ID_ACCEPT_KEY: u32 = 0x6fe1_735b;
const ID_ABORT_KEY: u32 = 0xdd05_ec6b;
const ID_COMMIT_KEY: u32 = 0xec2e_0b9b;
const ID_NOOP: u32 = 0xa82f_dd63;

const ID_SEND_TYPING: u32 = 0x16bf_744e;
const ID_SEND_CANCEL: u32 = 0xfd5e_c8f5;

/// Bytes aleatorios mínimos en `decryptedMessageLayer.random_bytes` (la especificación
/// obliga a ignorar mensajes con menos).
pub const MIN_RANDOM_BYTES: usize = 15;

// ---------------------------------------------------------------------------
// Escritura / lectura de primitivas

#[derive(Default)]
pub(crate) struct Writer {
    pub(crate) buf: Vec<u8>,
}

impl Writer {
    fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }
    fn bytes(&mut self, b: &[u8]) {
        let header = if b.len() < 254 {
            self.buf.push(b.len() as u8);
            1
        } else {
            self.buf.push(254);
            let l = b.len() as u32;
            self.buf.extend_from_slice(&l.to_le_bytes()[..3]);
            4
        };
        self.buf.extend_from_slice(b);
        let pad = (4 - (header + b.len()) % 4) % 4;
        self.buf.extend(core::iter::repeat_n(0u8, pad));
    }
    fn string(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }
    fn longs(&mut self, v: &[i64]) {
        self.u32(ID_VECTOR);
        self.i32(v.len() as i32);
        for x in v {
            self.i64(*x);
        }
    }
}

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(SecretError::Tl("desbordamiento"))?;
        let s = self.data.get(self.pos..end).ok_or(SecretError::Tl("datos truncados"))?;
        self.pos = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn i64(&mut self) -> Result<i64> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(i64::from_le_bytes(a))
    }
    fn bytes(&mut self) -> Result<Vec<u8>> {
        let first = self.take(1)?[0];
        let (len, header) = if first < 254 {
            (first as usize, 1)
        } else if first == 254 {
            let b = self.take(3)?;
            (b[0] as usize | (b[1] as usize) << 8 | (b[2] as usize) << 16, 4)
        } else {
            return Err(SecretError::Tl("longitud de bytes inválida"));
        };
        let out = self.take(len)?.to_vec();
        let pad = (4 - (header + len) % 4) % 4;
        self.take(pad)?;
        Ok(out)
    }
    fn string(&mut self) -> Result<String> {
        String::from_utf8(self.bytes()?).map_err(|_| SecretError::Tl("cadena no UTF-8"))
    }
    fn longs(&mut self) -> Result<Vec<i64>> {
        if self.u32()? != ID_VECTOR {
            return Err(SecretError::Tl("se esperaba Vector"));
        }
        let n = self.i32()?;
        let n = usize::try_from(n).map_err(|_| SecretError::Tl("longitud de vector negativa"))?;
        if n > (self.data.len() - self.pos) / 8 {
            return Err(SecretError::Tl("vector más largo que los datos"));
        }
        (0..n).map(|_| self.i64()).collect()
    }
    fn finish(&self) -> Result<()> {
        if self.pos == self.data.len() { Ok(()) } else { Err(SecretError::Tl("bytes sobrantes")) }
    }
}

// ---------------------------------------------------------------------------
// Tipos

/// `decryptedMessageLayer`: envoltorio con números de secuencia de todo mensaje moderno.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedMessageLayer {
    /// Al menos [`MIN_RANDOM_BYTES`] bytes aleatorios.
    pub random_bytes: Vec<u8>,
    /// Capa en la que está escrito `message`.
    pub layer: i32,
    /// `in_seq_no` en forma de cable (2·n + x).
    pub in_seq_no: i32,
    /// `out_seq_no` en forma de cable (2·n + x).
    pub out_seq_no: i32,
    /// Contenido.
    pub message: DecryptedMessage,
}

/// `DecryptedMessage`: mensaje normal o de servicio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptedMessage {
    /// `decryptedMessage#91cc4674`.
    Message(PlainMessage),
    /// `decryptedMessageService#73164160`.
    Service {
        /// Id aleatorio del mensaje (el mismo que se pasa a `sendEncryptedService`).
        random_id: i64,
        /// Acción.
        action: Action,
    },
}

impl DecryptedMessage {
    /// `random_id` del mensaje.
    pub fn random_id(&self) -> i64 {
        match self {
            DecryptedMessage::Message(m) => m.random_id,
            DecryptedMessage::Service { random_id, .. } => *random_id,
        }
    }
}

/// Contenido de `decryptedMessage` (capa 73).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlainMessage {
    /// `flags.1`: no generar vista previa de enlaces.
    pub no_webpage: bool,
    /// `flags.5`: sin notificación.
    pub silent: bool,
    /// Id aleatorio (el mismo que se pasa a `sendEncrypted`).
    pub random_id: i64,
    /// Segundos de vida tras leerse (0 = sin autodestrucción).
    pub ttl: i32,
    /// Texto.
    pub message: String,
    /// `flags.9`. Solo [`Media::Empty`].
    pub media: Option<Media>,
    /// `flags.7`. Vacío = campo ausente.
    pub entities: Vec<Entity>,
    /// `flags.11`.
    pub via_bot_name: Option<String>,
    /// `flags.3`.
    pub reply_to_random_id: Option<i64>,
    /// `flags.17`.
    pub grouped_id: Option<i64>,
}

/// Medios soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Media {
    /// `decryptedMessageMediaEmpty`.
    Empty,
}

/// Entidad de texto (`MessageEntity` de la capa secreta 144).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entity {
    /// Tipo.
    pub kind: EntityKind,
    /// Desplazamiento en unidades UTF-16.
    pub offset: i32,
    /// Longitud en unidades UTF-16.
    pub length: i32,
}

/// Tipos de entidad.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum EntityKind {
    Unknown,
    Mention,
    Hashtag,
    BotCommand,
    Url,
    Email,
    Bold,
    Italic,
    Code,
    Pre { language: String },
    TextUrl { url: String },
    Underline,
    Strike,
    Blockquote,
    Spoiler,
    CustomEmoji { document_id: i64 },
}

impl EntityKind {
    fn id(&self) -> u32 {
        match self {
            EntityKind::Unknown => 0xbb92_ba95,
            EntityKind::Mention => 0xfa04_579d,
            EntityKind::Hashtag => 0x6f63_5b0d,
            EntityKind::BotCommand => 0x6cef_8ac7,
            EntityKind::Url => 0x6ed0_2538,
            EntityKind::Email => 0x64e4_75c2,
            EntityKind::Bold => 0xbd61_0bc9,
            EntityKind::Italic => 0x826f_8b60,
            EntityKind::Code => 0x28a2_0571,
            EntityKind::Pre { .. } => 0x7392_4be0,
            EntityKind::TextUrl { .. } => 0x76a6_d327,
            EntityKind::Underline => 0x9c4e_7e8b,
            EntityKind::Strike => 0xbf06_93d4,
            EntityKind::Blockquote => 0x020d_f5d0,
            EntityKind::Spoiler => 0x32ca_960f,
            EntityKind::CustomEmoji { .. } => 0xc8cf_05f8,
        }
    }
}

/// Indicador de escritura (`SendMessageAction`), subconjunto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingAction {
    /// `sendMessageTypingAction`.
    Typing,
    /// `sendMessageCancelAction`.
    Cancel,
}

/// `DecryptedMessageAction` soportadas (los campos llevan el nombre del esquema TL).
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Temporizador de autodestrucción del chat, en segundos (0 = desactivado).
    SetMessageTtl { ttl_seconds: i32 },
    /// El otro leyó estos mensajes (arranca su temporizador).
    ReadMessages { random_ids: Vec<i64> },
    /// Borrar estos mensajes.
    DeleteMessages { random_ids: Vec<i64> },
    /// El otro hizo una captura de pantalla.
    ScreenshotMessages { random_ids: Vec<i64> },
    /// Vaciar historial.
    FlushHistory,
    /// Reenviar el rango `[start, end]` (forma de cable, extremos incluidos).
    Resend { start_seq_no: i32, end_seq_no: i32 },
    /// Capa máxima que soporta el emisor.
    NotifyLayer { layer: i32 },
    /// Escribiendo… / cancelar.
    Typing(TypingAction),
    /// Re-keying (PFS) paso 1.
    RequestKey { exchange_id: i64, g_a: Vec<u8> },
    /// Re-keying paso 2.
    AcceptKey { exchange_id: i64, g_b: Vec<u8>, key_fingerprint: i64 },
    /// Re-keying: abortar.
    AbortKey { exchange_id: i64 },
    /// Re-keying paso 3.
    CommitKey { exchange_id: i64, key_fingerprint: i64 },
    /// Sin efecto (sirve para estrenar una clave nueva).
    Noop,
}

/// Mensaje de nivel superior tras descifrar.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopLevel {
    /// Envoltorio moderno con números de secuencia.
    Layer(DecryptedMessageLayer),
    /// `decryptedMessageService#aa48327d` de capa 8 (sin secuencia), usado para `NotifyLayer`.
    LegacyService { random_id: i64, random_bytes: Vec<u8>, action: Action },
}

// ---------------------------------------------------------------------------
// Serialización

fn write_action(w: &mut Writer, a: &Action) {
    match a {
        Action::SetMessageTtl { ttl_seconds } => {
            w.u32(ID_SET_TTL);
            w.i32(*ttl_seconds);
        }
        Action::ReadMessages { random_ids } => {
            w.u32(ID_READ);
            w.longs(random_ids);
        }
        Action::DeleteMessages { random_ids } => {
            w.u32(ID_DELETE);
            w.longs(random_ids);
        }
        Action::ScreenshotMessages { random_ids } => {
            w.u32(ID_SCREENSHOT);
            w.longs(random_ids);
        }
        Action::FlushHistory => w.u32(ID_FLUSH),
        Action::Resend { start_seq_no, end_seq_no } => {
            w.u32(ID_RESEND);
            w.i32(*start_seq_no);
            w.i32(*end_seq_no);
        }
        Action::NotifyLayer { layer } => {
            w.u32(ID_NOTIFY_LAYER);
            w.i32(*layer);
        }
        Action::Typing(t) => {
            w.u32(ID_TYPING);
            w.u32(match t {
                TypingAction::Typing => ID_SEND_TYPING,
                TypingAction::Cancel => ID_SEND_CANCEL,
            });
        }
        Action::RequestKey { exchange_id, g_a } => {
            w.u32(ID_REQUEST_KEY);
            w.i64(*exchange_id);
            w.bytes(g_a);
        }
        Action::AcceptKey { exchange_id, g_b, key_fingerprint } => {
            w.u32(ID_ACCEPT_KEY);
            w.i64(*exchange_id);
            w.bytes(g_b);
            w.i64(*key_fingerprint);
        }
        Action::AbortKey { exchange_id } => {
            w.u32(ID_ABORT_KEY);
            w.i64(*exchange_id);
        }
        Action::CommitKey { exchange_id, key_fingerprint } => {
            w.u32(ID_COMMIT_KEY);
            w.i64(*exchange_id);
            w.i64(*key_fingerprint);
        }
        Action::Noop => w.u32(ID_NOOP),
    }
}

fn read_action(r: &mut Reader<'_>) -> Result<Action> {
    let id = r.u32()?;
    Ok(match id {
        ID_SET_TTL => Action::SetMessageTtl { ttl_seconds: r.i32()? },
        ID_READ => Action::ReadMessages { random_ids: r.longs()? },
        ID_DELETE => Action::DeleteMessages { random_ids: r.longs()? },
        ID_SCREENSHOT => Action::ScreenshotMessages { random_ids: r.longs()? },
        ID_FLUSH => Action::FlushHistory,
        ID_RESEND => Action::Resend { start_seq_no: r.i32()?, end_seq_no: r.i32()? },
        ID_NOTIFY_LAYER => Action::NotifyLayer { layer: r.i32()? },
        ID_TYPING => Action::Typing(match r.u32()? {
            ID_SEND_TYPING => TypingAction::Typing,
            ID_SEND_CANCEL => TypingAction::Cancel,
            other => return Err(SecretError::UnsupportedConstructor(other)),
        }),
        ID_REQUEST_KEY => Action::RequestKey { exchange_id: r.i64()?, g_a: r.bytes()? },
        ID_ACCEPT_KEY => Action::AcceptKey { exchange_id: r.i64()?, g_b: r.bytes()?, key_fingerprint: r.i64()? },
        ID_ABORT_KEY => Action::AbortKey { exchange_id: r.i64()? },
        ID_COMMIT_KEY => Action::CommitKey { exchange_id: r.i64()?, key_fingerprint: r.i64()? },
        ID_NOOP => Action::Noop,
        other => return Err(SecretError::UnsupportedConstructor(other)),
    })
}

fn write_entity(w: &mut Writer, e: &Entity) {
    w.u32(e.kind.id());
    w.i32(e.offset);
    w.i32(e.length);
    match &e.kind {
        EntityKind::Pre { language } => w.string(language),
        EntityKind::TextUrl { url } => w.string(url),
        EntityKind::CustomEmoji { document_id } => w.i64(*document_id),
        _ => {}
    }
}

fn read_entity(r: &mut Reader<'_>) -> Result<Entity> {
    let id = r.u32()?;
    let offset = r.i32()?;
    let length = r.i32()?;
    let kind = match id {
        0xbb92_ba95 => EntityKind::Unknown,
        0xfa04_579d => EntityKind::Mention,
        0x6f63_5b0d => EntityKind::Hashtag,
        0x6cef_8ac7 => EntityKind::BotCommand,
        0x6ed0_2538 => EntityKind::Url,
        0x64e4_75c2 => EntityKind::Email,
        0xbd61_0bc9 => EntityKind::Bold,
        0x826f_8b60 => EntityKind::Italic,
        0x28a2_0571 => EntityKind::Code,
        0x7392_4be0 => EntityKind::Pre { language: r.string()? },
        0x76a6_d327 => EntityKind::TextUrl { url: r.string()? },
        0x9c4e_7e8b => EntityKind::Underline,
        0xbf06_93d4 => EntityKind::Strike,
        0x020d_f5d0 => EntityKind::Blockquote,
        0x32ca_960f => EntityKind::Spoiler,
        0xc8cf_05f8 => EntityKind::CustomEmoji { document_id: r.i64()? },
        other => return Err(SecretError::UnsupportedConstructor(other)),
    };
    Ok(Entity { kind, offset, length })
}

fn write_message(w: &mut Writer, m: &DecryptedMessage) {
    match m {
        DecryptedMessage::Message(p) => {
            let mut flags = 0u32;
            if p.no_webpage {
                flags |= 1 << 1;
            }
            if p.reply_to_random_id.is_some() {
                flags |= 1 << 3;
            }
            if p.silent {
                flags |= 1 << 5;
            }
            if !p.entities.is_empty() {
                flags |= 1 << 7;
            }
            if p.media.is_some() {
                flags |= 1 << 9;
            }
            if p.via_bot_name.is_some() {
                flags |= 1 << 11;
            }
            if p.grouped_id.is_some() {
                flags |= 1 << 17;
            }
            w.u32(ID_MESSAGE);
            w.u32(flags);
            w.i64(p.random_id);
            w.i32(p.ttl);
            w.string(&p.message);
            if let Some(Media::Empty) = p.media {
                w.u32(ID_MEDIA_EMPTY);
            }
            if !p.entities.is_empty() {
                w.u32(ID_VECTOR);
                w.i32(p.entities.len() as i32);
                for e in &p.entities {
                    write_entity(w, e);
                }
            }
            if let Some(v) = &p.via_bot_name {
                w.string(v);
            }
            if let Some(r) = p.reply_to_random_id {
                w.i64(r);
            }
            if let Some(g) = p.grouped_id {
                w.i64(g);
            }
        }
        DecryptedMessage::Service { random_id, action } => {
            w.u32(ID_SERVICE);
            w.i64(*random_id);
            write_action(w, action);
        }
    }
}

fn read_message(r: &mut Reader<'_>) -> Result<DecryptedMessage> {
    let id = r.u32()?;
    match id {
        ID_MESSAGE => {
            let flags = r.u32()?;
            let has = |bit: u32| flags & (1 << bit) != 0;
            let mut p = PlainMessage {
                no_webpage: has(1),
                silent: has(5),
                random_id: r.i64()?,
                ttl: r.i32()?,
                message: r.string()?,
                ..PlainMessage::default()
            };
            if has(9) {
                p.media = Some(match r.u32()? {
                    ID_MEDIA_EMPTY => Media::Empty,
                    other => return Err(SecretError::UnsupportedConstructor(other)),
                });
            }
            if has(7) {
                if r.u32()? != ID_VECTOR {
                    return Err(SecretError::Tl("se esperaba Vector"));
                }
                let n = usize::try_from(r.i32()?).map_err(|_| SecretError::Tl("longitud de vector negativa"))?;
                if n > 10_000 {
                    return Err(SecretError::Tl("demasiadas entidades"));
                }
                p.entities = (0..n).map(|_| read_entity(r)).collect::<Result<_>>()?;
            }
            if has(11) {
                p.via_bot_name = Some(r.string()?);
            }
            if has(3) {
                p.reply_to_random_id = Some(r.i64()?);
            }
            if has(17) {
                p.grouped_id = Some(r.i64()?);
            }
            Ok(DecryptedMessage::Message(p))
        }
        ID_SERVICE => Ok(DecryptedMessage::Service { random_id: r.i64()?, action: read_action(r)? }),
        other => Err(SecretError::UnsupportedConstructor(other)),
    }
}

impl DecryptedMessageLayer {
    /// Serializa a TL.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.u32(ID_LAYER);
        w.bytes(&self.random_bytes);
        w.i32(self.layer);
        w.i32(self.in_seq_no);
        w.i32(self.out_seq_no);
        write_message(&mut w, &self.message);
        w.buf
    }
}

impl TopLevel {
    /// Serializa a TL.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TopLevel::Layer(l) => l.to_bytes(),
            TopLevel::LegacyService { random_id, random_bytes, action } => {
                let mut w = Writer::default();
                w.u32(ID_SERVICE_L8);
                w.i64(*random_id);
                w.bytes(random_bytes);
                write_action(&mut w, action);
                w.buf
            }
        }
    }

    /// Analiza una carga descifrada completa (debe consumirse entera).
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut r = Reader::new(data);
        let out = match r.u32()? {
            ID_LAYER => {
                let random_bytes = r.bytes()?;
                let layer = r.i32()?;
                let in_seq_no = r.i32()?;
                let out_seq_no = r.i32()?;
                let message = read_message(&mut r)?;
                TopLevel::Layer(DecryptedMessageLayer { random_bytes, layer, in_seq_no, out_seq_no, message })
            }
            ID_SERVICE_L8 => {
                let random_id = r.i64()?;
                let random_bytes = r.bytes()?;
                let action = read_action(&mut r)?;
                TopLevel::LegacyService { random_id, random_bytes, action }
            }
            other => return Err(SecretError::UnsupportedConstructor(other)),
        };
        r.finish()?;
        Ok(out)
    }
}
