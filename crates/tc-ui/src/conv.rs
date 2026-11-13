//! Estado de la conversación abierta y matemática del scroll virtual (M1.2).
//!
//! iced 0.14 no trae lista virtual. Se usa un `scrollable` anclado abajo con espaciadores: cada
//! fila tiene una altura **medida** con el mismo motor de texto que el render (`Paragraph`) y se
//! pinta en un contenedor de esa altura exacta, así la suma de prefijos es la altura real del
//! contenido y solo se construyen las filas visibles (+ una pantalla de margen por lado).
//! Las alturas se guardan en caché por (mensaje, firma de contenido, ancho de columna).

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

use iced::Size;
use tc_core::{EntityKind, Message, MsgId, PeerRef, RandomId, SendState, UserId};
use tc_theme::TextStyle;
use tc_theme::tokens::{layout, space, text as ts};

use crate::style::GroupPos;

// ---------------------------------------------------------------- medidas (px lógicos)

pub const PAD_X: f32 = space::S3;
pub const PAD_Y: f32 = space::S2;
pub const GAP_IN_GROUP: f32 = space::S0_5;
pub const GAP_GROUP: f32 = space::S3;
pub const NAME_H: f32 = 18.0;
pub const FWD_H: f32 = 18.0;
pub const REPLY_H: f32 = 40.0;
pub const REPLY_GAP: f32 = space::S1;
pub const REPLY_MIN_W: f32 = 220.0;
pub const MEDIA_GAP: f32 = space::S1_5;
pub const MEDIA_MAX_W: f32 = 320.0;
pub const META_H: f32 = 16.0;
/// Elevación de la hora en línea para alinear su línea de base con la del texto del mensaje.
pub const META_BASELINE: f32 = 2.0;
pub const META_GAP: f32 = space::S0_5;
pub const META_PILL_H: f32 = 20.0;
pub const REACT_GAP: f32 = space::S1;
pub const REACT_H: f32 = 28.0;
/// Caja del sticker en la burbuja (los de la red miden 512 px; se dibujan a ~200 px como en los
/// clientes oficiales, conservando su proporción).
pub const STICKER: f32 = 200.0;
pub const DATE_H: f32 = 40.0;
pub const MARK_H: f32 = 32.0;
pub const EDGE_H: f32 = 48.0;
pub const SIDE_PAD: f32 = space::S4;
pub const GROUP_AVATAR: f32 = layout::AVATAR_SM;
pub const GROUP_AVATAR_COL: f32 = layout::AVATAR_SM + space::S2;
/// Icono de estado (check/reloj) y temporizador en la zona meta.
pub const META_ICON: f32 = 14.0;
pub const META_ICON_GAP: f32 = space::S1;
/// Agrupación: mismo remitente y menos de 5 minutos.
pub const GROUP_WINDOW_SECS: i64 = 300;
pub const PAGE: usize = 60;

/// Medidor de texto. En la app lo implementa el `Paragraph` de iced (mismo motor que el render);
/// en las pruebas, un medidor de ancho fijo por carácter.
pub trait Measurer {
    /// Tamaño mínimo del texto envuelto en `max_w` con el estilo dado.
    fn text(&self, content: &str, style: TextStyle, max_w: f32) -> Size;
    /// Igual, con tramos de formato (negrita, cursiva, código).
    fn rich(&self, spans: &[Run<'_>], style: TextStyle, max_w: f32) -> Size;
}

/// Tramo de texto con formato.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run<'a> {
    pub text: &'a str,
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub strike: bool,
    pub underline: bool,
    pub link: bool,
}

/// Parte el texto del mensaje en tramos según sus entidades (offsets en caracteres).
pub fn runs(m: &Message) -> Vec<Run<'_>> {
    let text = m.text.as_str();
    let plain = |t| Run { text: t, bold: false, italic: false, mono: false, strike: false, underline: false, link: false };
    if m.entities.is_empty() {
        return vec![plain(text)];
    }
    // Byte de inicio de cada carácter (+ final).
    let mut idx: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    idx.push(text.len());
    let n = idx.len() - 1;
    let mut cuts: Vec<usize> = vec![0, n];
    for e in &m.entities {
        let a = (e.offset as usize).min(n);
        let b = (e.offset as usize + e.length as usize).min(n);
        cuts.extend([a, b]);
    }
    cuts.sort_unstable();
    cuts.dedup();
    cuts.windows(2)
        .filter(|w| w[0] < w[1])
        .map(|w| {
            let mut r = plain(&text[idx[w[0]]..idx[w[1]]]);
            for e in &m.entities {
                let (a, b) = (e.offset as usize, e.offset as usize + e.length as usize);
                if a <= w[0] && w[1] <= b {
                    match e.kind {
                        EntityKind::Bold => r.bold = true,
                        EntityKind::Italic => r.italic = true,
                        EntityKind::Code | EntityKind::Pre => r.mono = true,
                        EntityKind::Strike => r.strike = true,
                        EntityKind::Underline => r.underline = true,
                        EntityKind::Mention | EntityKind::Url | EntityKind::Hashtag => r.link = true,
                        EntityKind::Spoiler => {}
                    }
                }
            }
            r
        })
        .collect()
}

/// ¿Solo 1–3 emojis? Se pinta sin burbuja y en grande.
pub fn emoji_only(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 12 {
        return false;
    }
    let pictos = t.chars().filter(|c| !c.is_ascii() && !matches!(*c, '\u{fe0f}' | '\u{200d}') && !c.is_alphanumeric()).count();
    t.chars().all(|c| !c.is_ascii() && !c.is_alphanumeric() || c == ' ') && (1..=3).contains(&pictos)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Body {
    Bubble,
    Sticker,
    Emoji,
}

/// Geometría intrínseca de un mensaje (no depende de sus vecinos salvo `show_name`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MsgLayout {
    pub body: Body,
    /// Caja del texto (ancho al que se fija el widget de texto para envolver igual que al medir).
    pub text: Size,
    pub content_w: f32,
    pub meta_inline: bool,
    pub meta_w: f32,
    pub media: Option<Size>,
    /// Tamaño de la burbuja (o del sticker/emoji con su meta).
    pub bubble: Size,
    pub show_name: bool,
    pub day: i32,
}

/// Datos de contexto para maquetar un mensaje.
pub struct LayoutInput<'a> {
    pub show_name: bool,
    pub name: &'a str,
    pub forwarded: Option<&'a str>,
    pub meta: &'a str,
    pub bubble_max: f32,
    pub day: i32,
}

pub fn meta_width(m: &Message, meta_label_w: f32) -> f32 {
    let mut w = meta_label_w;
    if m.outgoing {
        w += META_ICON_GAP + META_ICON;
    }
    if m.ttl_secs.is_some() {
        w += META_ICON_GAP + META_ICON;
    }
    w
}

pub fn layout_message(m: &Message, input: &LayoutInput<'_>, meas: &impl Measurer) -> MsgLayout {
    let inner_max = (input.bubble_max - 2.0 * PAD_X).max(40.0);
    let meta_w = meta_width(m, meas.text(input.meta, ts::CAPTION, f32::INFINITY).width);
    let base = MsgLayout {
        body: Body::Bubble,
        text: Size::ZERO,
        content_w: 0.0,
        meta_inline: false,
        meta_w,
        media: None,
        bubble: Size::ZERO,
        show_name: input.show_name,
        day: input.day,
    };
    if m.sticker.is_some() {
        let w = STICKER.max(meta_w + 2.0 * PAD_X);
        return MsgLayout { body: Body::Sticker, bubble: Size::new(w, STICKER + META_GAP + META_PILL_H), ..base };
    }
    if m.media.is_none() && m.reply_to.is_none() && m.forwarded_from.is_none() && emoji_only(&m.text) {
        let t = meas.text(m.text.trim(), ts::EMOJI_ONLY, inner_max);
        let w = t.width.max(meta_w + 2.0 * PAD_X);
        return MsgLayout { body: Body::Emoji, text: t, bubble: Size::new(w, t.height + META_GAP + META_PILL_H), ..base };
    }

    let mut h = 2.0 * PAD_Y;
    let mut content_w: f32 = 0.0;
    if input.show_name {
        h += NAME_H;
        content_w = content_w.max(meas.text(input.name, crate::style::with_weight(ts::META, 600), inner_max).width);
    }
    if let Some(f) = input.forwarded {
        h += FWD_H;
        content_w = content_w.max(meas.text(f, ts::META, inner_max).width.min(inner_max));
    }
    if m.reply_to.is_some() {
        h += REPLY_H + REPLY_GAP;
        content_w = content_w.max(REPLY_MIN_W.min(inner_max));
    }
    let media = m.media.as_ref().map(|md| {
        // Notas de voz, audios y vídeos redondos tienen su propio tamaño.
        if let Some(sz) = crate::audio::layout_size(md, inner_max) {
            return sz;
        }
        let w = MEDIA_MAX_W.min(inner_max);
        let ratio = if md.width > 0 { md.height as f32 / md.width as f32 } else { 0.75 };
        Size::new(w, (w * ratio).clamp(120.0, 360.0).round())
    });
    if let Some(sz) = media {
        h += sz.height;
        content_w = content_w.max(sz.width);
    }
    let has_text = !m.text.trim().is_empty();
    let mut text = Size::ZERO;
    let mut meta_inline = false;
    if has_text {
        if media.is_some() {
            h += MEDIA_GAP;
        }
        let rs = runs(m);
        text = if m.entities.is_empty() { meas.text(&m.text, ts::BODY, inner_max) } else { meas.rich(&rs, ts::BODY, inner_max) };
        text.width = text.width.ceil().min(inner_max);
        text.height = text.height.max(ts::BODY.line);
        let one_line = text.height <= ts::BODY.line + 0.5;
        meta_inline = one_line && text.width + space::S2 + meta_w <= inner_max;
        h += text.height;
        content_w = content_w.max(if meta_inline { text.width + space::S2 + meta_w } else { text.width });
    }
    if !meta_inline {
        h += META_GAP + META_H;
        content_w = content_w.max(meta_w);
    }
    let bubble = Size::new((content_w + 2.0 * PAD_X).ceil(), h.ceil());
    MsgLayout { text, content_w, meta_inline, media, bubble, ..base }
}

// ---------------------------------------------------------------- ventana virtual

/// Rango de elementos a construir y alturas de los espaciadores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub start: usize,
    pub end: usize,
    pub top_space: f32,
    pub bottom_space: f32,
}

/// `prefix[i]` = altura acumulada antes del elemento `i` (longitud n + 1).
/// `from_bottom` es la distancia del borde inferior de la vista al final del contenido.
pub fn visible_window(prefix: &[f32], viewport_h: f32, from_bottom: f32, overscan: f32) -> Window {
    let n = prefix.len().saturating_sub(1);
    let total = prefix.last().copied().unwrap_or(0.0);
    let from_top = (total - viewport_h - from_bottom).max(0.0);
    let lo = from_top - overscan;
    let hi = from_top + viewport_h + overscan;
    // Primer elemento cuyo final queda por debajo de `lo`.
    let start = prefix[1..].partition_point(|end| *end <= lo).min(n);
    // Elementos cuyo inicio queda por encima de `hi`.
    let end = prefix[..n].partition_point(|s| *s < hi).max(start);
    Window { start, end, top_space: prefix[start], bottom_space: total - prefix[end] }
}

// ---------------------------------------------------------------- estado

/// Clave estable de un mensaje para animaciones: el `random_id` sobrevive al cambio de id
/// cuando el pendiente pasa a enviado.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MsgKey {
    Random(RandomId),
    Id(MsgId),
}

impl MsgKey {
    pub fn of(m: &Message) -> Self {
        m.random_id.map_or(Self::Id(m.id), Self::Random)
    }
}

/// Id local de los pendientes: negativo, nunca choca con los del servidor.
pub fn is_local(m: &Message) -> bool {
    m.id.0 <= 0
}

/// Elemento de la lista virtual.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item {
    /// Fin del historial (o «cargando más»).
    Edge { loading: bool },
    Date { ts: tc_core::Timestamp },
    UnreadMark,
    Msg { idx: usize, pos: GroupPos, gap: f32, avatar: bool, height: f32 },
}

impl Item {
    pub fn height(&self) -> f32 {
        match self {
            Item::Edge { .. } => EDGE_H,
            Item::Date { .. } => DATE_H,
            Item::UnreadMark => MARK_H,
            Item::Msg { height, .. } => *height,
        }
    }
}

#[derive(Debug, Default)]
pub struct LayoutCache {
    pub width: f32,
    map: HashMap<(i32, i64), (u64, MsgLayout)>,
    /// Día local de cada mensaje (la conversión de zona horaria se hace una sola vez).
    days: HashMap<(i32, i64), i32>,
}

impl LayoutCache {
    pub fn day(&mut self, m: &Message, f: impl FnOnce() -> i32) -> i32 {
        *self.days.entry((m.id.0, m.random_id.map_or(0, |r| r.0))).or_insert_with(f)
    }

    pub fn reset_if_width(&mut self, width: f32) {
        if (self.width - width).abs() > 0.5 {
            self.map.clear();
            self.width = width;
        }
    }

    pub fn get_or(&mut self, m: &Message, sig: u64, f: impl FnOnce() -> MsgLayout) -> MsgLayout {
        let key = (m.id.0, m.random_id.map_or(0, |r| r.0));
        match self.map.get(&key) {
            Some((s, l)) if *s == sig => *l,
            _ => {
                let l = f();
                self.map.insert(key, (sig, l));
                l
            }
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Firma de lo que afecta a la altura de un mensaje.
pub fn signature(m: &Message, show_name: bool) -> u64 {
    let mut h = DefaultHasher::new();
    m.text.hash(&mut h);
    m.entities.len().hash(&mut h);
    m.edit_date.hash(&mut h);
    m.reply_to.hash(&mut h);
    m.forwarded_from.hash(&mut h);
    m.media.as_ref().map(|x| (x.width, x.height)).hash(&mut h);
    m.sticker.as_ref().map(|s| s.id).hash(&mut h);
    m.ttl_secs.is_some().hash(&mut h);
    show_name.hash(&mut h);
    h.finish()
}

/// ¿Va `b` en el mismo grupo visual que `a` (anterior)?
pub fn same_group(a: &Message, b: &Message, day_a: i32, day_b: i32) -> bool {
    a.outgoing == b.outgoing && a.sender == b.sender && day_a == day_b && (b.date.0 - a.date.0).abs() < GROUP_WINDOW_SECS
}

/// Conversación abierta: ventana en memoria de mensajes en orden ascendente; los pendientes
/// (id local negativo) van siempre al final.
#[derive(Debug)]
pub struct Conversation {
    pub peer: PeerRef,
    pub messages: Vec<Message>,
    pub loading: bool,
    pub loading_older: bool,
    pub reached_start: bool,
    pub error: Option<String>,
    /// Distancia del borde inferior de la vista al final del contenido.
    pub from_bottom: f32,
    pub from_top: f32,
    /// Mensajes llegados mientras se leía más arriba.
    pub new_below: u32,
    /// Primer mensaje no leído al abrir (para la marca).
    pub unread_from: Option<MsgId>,
    pub cache: std::cell::RefCell<LayoutCache>,
    /// Ancho de la columna de la última maquetación (para compensar el scroll fuera de `view`).
    pub last_col: std::cell::Cell<f32>,
    pub last_viewport: std::cell::Cell<f32>,
    /// Hay algún sticker animado en la ventana visible (lo fija la vista; decide si se piden frames).
    pub anim_visible: std::cell::Cell<bool>,
    /// Índices `[desde, hasta)` de los mensajes de la ventana visible (lo fija la vista; decide qué
    /// fotos se cargan).
    pub visible: std::cell::Cell<Option<(usize, usize)>>,
    /// Lectura: ventana hacia lo más nuevo, salto a la marca y marcado progresivo (`reading.rs`).
    pub reading: crate::reading::ReadState,
}

impl Conversation {
    pub fn new(peer: PeerRef) -> Self {
        Self {
            peer,
            messages: Vec::new(),
            loading: true,
            loading_older: false,
            reached_start: false,
            error: None,
            from_bottom: 0.0,
            from_top: 0.0,
            new_below: 0,
            unread_from: None,
            cache: Default::default(),
            last_col: std::cell::Cell::new(0.0),
            last_viewport: std::cell::Cell::new(0.0),
            anim_visible: std::cell::Cell::new(false),
            visible: std::cell::Cell::new(None),
            reading: Default::default(),
        }
    }

    /// Mensajes cuyas fotos interesan: la ventana visible con un margen, o los últimos antes de la
    /// primera maquetación (al abrir se ve el final).
    pub fn image_range(&self) -> (usize, usize) {
        const MARGIN: usize = 6;
        let n = self.messages.len();
        match self.visible.get() {
            Some((a, z)) => (a.saturating_sub(MARGIN).min(n), z.saturating_add(MARGIN).min(n)),
            None => (n.saturating_sub(20), n),
        }
    }

    pub fn oldest_real(&self) -> Option<MsgId> {
        self.messages.iter().find(|m| !is_local(m)).map(|m| m.id)
    }

    pub fn newest_real(&self) -> Option<MsgId> {
        self.messages.iter().rev().find(|m| !is_local(m)).map(|m| m.id)
    }

    /// Inserta o sustituye un mensaje del servidor en su sitio (idempotente por id) y retira el
    /// pendiente con el mismo `random_id`. Devuelve `true` si era nuevo.
    pub fn upsert(&mut self, m: Message) -> bool {
        if let Some(r) = m.random_id {
            self.messages.retain(|x| !(is_local(x) && x.random_id == Some(r)));
        }
        if let Some(existing) = self.messages.iter_mut().find(|x| !is_local(x) && x.id == m.id) {
            *existing = m;
            return false;
        }
        let pos = self.messages.iter().position(|x| is_local(x) || x.id > m.id).unwrap_or(self.messages.len());
        self.messages.insert(pos, m);
        true
    }

    /// Antepone una página de historial antiguo (ascendente), sin duplicados.
    pub fn prepend(&mut self, page: Vec<Message>) {
        let first = self.oldest_real();
        let older: Vec<Message> = page.into_iter().filter(|m| first.is_none_or(|f| m.id < f)).collect();
        self.messages.splice(0..0, older);
    }

    pub fn push_pending(&mut self, m: Message) {
        self.messages.push(m);
    }

    pub fn set_state(&mut self, random_id: RandomId, state: SendState) {
        if let Some(m) = self.messages.iter_mut().find(|m| m.random_id == Some(random_id) && is_local(m)) {
            m.state = state;
        }
    }

    pub fn remove(&mut self, ids: &[MsgId]) {
        self.messages.retain(|m| is_local(m) || !ids.contains(&m.id));
    }

    pub fn find(&self, id: MsgId) -> Option<&Message> {
        self.messages.iter().find(|m| m.id == id)
    }

    pub fn find_mut(&mut self, id: MsgId) -> Option<&mut Message> {
        self.messages.iter_mut().find(|m| m.id == id)
    }

    pub fn last_own(&self, me: Option<UserId>) -> Option<&Message> {
        self.messages.iter().rev().find(|m| !is_local(m) && m.outgoing && (me.is_none() || m.sender == me) && m.sticker.is_none())
    }

    pub fn near_bottom(&self) -> bool {
        self.from_bottom < 48.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tc_core::*;

    struct Fixed;
    impl Measurer for Fixed {
        fn text(&self, content: &str, style: TextStyle, max_w: f32) -> Size {
            let cw = style.size * 0.5;
            let total = content.chars().count() as f32 * cw;
            let lines = (total / max_w).ceil().max(1.0);
            Size::new(total.min(max_w), lines * style.line)
        }
        fn rich(&self, spans: &[Run<'_>], style: TextStyle, max_w: f32) -> Size {
            let s: String = spans.iter().map(|r| r.text).collect();
            self.text(&s, style, max_w)
        }
    }

    fn msg(id: i32, text: &str, outgoing: bool) -> Message {
        Message {
            id: MsgId(id),
            peer: PeerRef::User(UserId(1)),
            sender: Some(UserId(if outgoing { 0 } else { 1 })),
            date: Timestamp(1000 + id as i64),
            edit_date: None,
            outgoing,
            text: text.into(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        }
    }

    fn input(meta: &str) -> LayoutInput<'_> {
        LayoutInput { show_name: false, name: "", forwarded: None, meta, bubble_max: 560.0, day: 0 }
    }

    #[test]
    fn ventana_virtual_abajo_y_arriba() {
        // 1000 elementos de 50 px.
        let prefix: Vec<f32> = (0..=1000).map(|i| i as f32 * 50.0).collect();
        let w = visible_window(&prefix, 500.0, 0.0, 500.0);
        assert_eq!(w.end, 1000, "anclado abajo: el último se construye");
        assert!(w.start >= 1000 - 21 && w.start <= 1000 - 19, "≈ 2 pantallas: {}", w.start);
        assert_eq!(w.top_space, prefix[w.start]);
        assert_eq!(w.bottom_space, 0.0);
        let w = visible_window(&prefix, 500.0, 50_000.0 - 500.0, 500.0);
        assert_eq!(w.start, 0, "arriba del todo");
        assert!(w.end <= 21);
        // Suma constante: espaciadores + filas = total.
        let w = visible_window(&prefix, 500.0, 20_000.0, 500.0);
        let rows = prefix[w.end] - prefix[w.start];
        assert!((w.top_space + rows + w.bottom_space - 50_000.0).abs() < 0.01);
        assert!(w.end - w.start <= 32, "solo ~3 pantallas de filas: {}", w.end - w.start);
        let empty = visible_window(&[0.0], 500.0, 0.0, 500.0);
        assert_eq!((empty.start, empty.end), (0, 0));
    }

    #[test]
    fn maquetacion_meta_en_linea_o_debajo() {
        let corto = layout_message(&msg(1, "hola", true), &input("12:00"), &Fixed);
        assert!(corto.meta_inline);
        assert_eq!(corto.bubble.height, 2.0 * PAD_Y + 22.0);
        let largo = layout_message(&msg(2, &"palabra ".repeat(40), true), &input("12:00"), &Fixed);
        assert!(!largo.meta_inline);
        assert!(largo.bubble.height > corto.bubble.height + 22.0);
        assert!(largo.bubble.width <= 560.0);
        let emoji = layout_message(&msg(3, "🔥🔥", false), &input("12:00"), &Fixed);
        assert_eq!(emoji.body, Body::Emoji);
        assert!(!emoji_only("hola 🔥"));
        assert!(emoji_only("❤️"));
    }

    #[test]
    fn pendiente_se_sustituye_por_el_real() {
        let mut c = Conversation::new(PeerRef::User(UserId(1)));
        c.prepend((1..=3).map(|i| msg(i, "x", false)).collect());
        let mut p = msg(-1, "nuevo", true);
        p.random_id = Some(RandomId(77));
        p.state = SendState::Pending;
        c.push_pending(p);
        // Llega un entrante antes de la confirmación: va antes del pendiente.
        assert!(c.upsert(msg(4, "entrante", false)));
        assert_eq!(c.messages.iter().map(|m| m.id.0).collect::<Vec<_>>(), vec![1, 2, 3, 4, -1]);
        let mut real = msg(5, "nuevo", true);
        real.random_id = Some(RandomId(77));
        assert!(c.upsert(real.clone()));
        assert!(!c.upsert(real), "idempotente");
        assert_eq!(c.messages.iter().map(|m| m.id.0).collect::<Vec<_>>(), vec![1, 2, 3, 4, 5]);
        c.prepend((1..=3).map(|i| msg(i, "dup", false)).collect());
        assert_eq!(c.messages.len(), 5, "sin duplicados al anteponer");
        c.remove(&[MsgId(2)]);
        assert_eq!(c.messages.len(), 4);
        assert_eq!(c.oldest_real(), Some(MsgId(1)));
        assert_eq!(c.newest_real(), Some(MsgId(5)));
    }

    #[test]
    fn tramos_por_entidades() {
        let mut m = msg(1, "Hola mundo ñandú", false);
        m.entities.push(TextEntity { kind: EntityKind::Bold, offset: 0, length: 4 });
        m.entities.push(TextEntity { kind: EntityKind::Code, offset: 11, length: 5 });
        let r = runs(&m);
        assert_eq!(r.iter().map(|r| r.text).collect::<String>(), m.text);
        assert!(r[0].bold && r[0].text == "Hola");
        assert!(r.last().unwrap().mono && r.last().unwrap().text == "ñandú");
    }

    #[test]
    fn radios_de_union() {
        use crate::style::bubble_radius;
        let r = bubble_radius(true, GroupPos::Middle);
        assert_eq!((r.top_right, r.bottom_right, r.top_left), (6.0, 6.0, 18.0));
        let r = bubble_radius(false, GroupPos::First);
        assert_eq!((r.top_left, r.bottom_left, r.top_right), (18.0, 6.0, 18.0));
        assert_eq!(bubble_radius(true, GroupPos::Single).bottom_right, 18.0);
    }
}
