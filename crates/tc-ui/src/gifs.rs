//! GIFs en la UI (ADR-0010): pestaña del panel (guardados, búsqueda en lo guardado y en la red) y
//! animación en bucle, sin sonido, de todos los GIF.
//!
//! * Los GIF de Telegram son casi siempre MP4/H.264 sin sonido (`documentAttributeAnimated`): se
//!   decodifican con tc-video (demultiplexor MP4 propio + OpenH264).
//! * Los GIF de verdad (`image/gif`) se decodifican con `image`.
//! * En los dos casos: descarga entera (como mucho `MAX_ANIM_BYTES`), decodificado fuera del hilo de
//!   la UI, fotogramas de como mucho `ANIM_PX` y `MAX_FRAMES`, y un presupuesto de memoria por
//!   animación (`ANIM_BUDGET`, ver `tc_video::plan`) para que las `MAX_ANIMATED` animaciones a la
//!   vista quepan juntas en la LRU (`CACHE_BYTES`) sin desalojarse unas a otras. Se animan con el
//!   reloj compartido de los stickers (`App::sticker_ms`, `sticker_anim::TICK`); sin foco o con
//!   movimiento reducido, quietas (primer fotograma).
//! * Lo que no se puede decodificar (otro códec, archivo dañado) se queda con su miniatura y la
//!   insignia «GIF».

use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ::image::AnimationDecoder;
use ::image::RgbaImage;
use ::image::codecs::gif::GifDecoder;
use ::image::imageops::FilterType;
use iced::widget::image::Handle;
use tc_core::{BackendResult, Gif, GifPage, Media, MediaKind, RandomId};
use tc_video::Limits;
use tc_video::plan;

use crate::format::fold;
use crate::stickers::Picker;

/// GIFs más grandes que esto no se animan (se queda la miniatura).
pub const MAX_ANIM_BYTES: u64 = 8 * 1024 * 1024;
/// Lado máximo (px físicos) de los fotogramas animados.
pub const ANIM_PX: u32 = 320;
/// Lado por debajo del cual, para caber en el presupuesto, se prefiere quitar fotogramas a encoger.
pub const MIN_ANIM_PX: u32 = 160;
/// Fotogramas como mucho por GIF (si hay más, se reparten por todo el bucle).
pub const MAX_FRAMES: usize = 150;
/// Tope de memoria de todos los fotogramas decodificados.
pub const CACHE_BYTES: usize = 128 * 1024 * 1024;
/// GIFs animándose a la vez como mucho (los demás, miniatura con insignia).
pub const MAX_ANIMATED: usize = 12;
/// Presupuesto de cada animación: así las `MAX_ANIMATED` a la vista caben juntas en la caché.
pub const ANIM_BUDGET: usize = CACHE_BYTES / MAX_ANIMATED;
const _: () = assert!(MAX_ANIMATED * ANIM_BUDGET <= CACHE_BYTES, "las animaciones a la vista caben juntas");
/// Topes del decodificado. Los fotogramas más cortos que el reloj (`sticker_anim::TICK`) se agrupan.
pub const LIMITS: Limits = Limits {
    max_side: ANIM_PX,
    min_side: MIN_ANIM_PX,
    max_frames: MAX_FRAMES,
    max_bytes: ANIM_BUDGET,
    min_frame_ms: crate::sticker_anim::TICK.as_millis() as u32,
};
/// Animaciones que se cargan a la vez.
pub const MAX_INFLIGHT: usize = 3;
/// Retardo mínimo de un fotograma (los GIF con 0 o 10 ms se ven a 10 fps en los navegadores).
pub const MIN_DELAY_MS: u32 = 20;
const DEFAULT_DELAY_MS: u32 = 100;
const RETRY_AFTER: Duration = Duration::from_secs(30);
/// Celda de la cuenta de la rejilla del panel.
pub const CELL_H: f32 = 96.0;
pub const CELL_GAP: f32 = 4.0;
pub const CELL_MIN_W: f32 = 120.0;
/// Alto supuesto de la rejilla hasta que el `scrollable` informa del suyo.
const DEFAULT_VIEW_H: f32 = 4.0 * (CELL_H + CELL_GAP);

#[derive(Debug, Clone)]
pub enum GifMsg {
    /// Llegó la lista de guardados.
    Saved(BackendResult<Vec<Gif>>),
    /// Texto del buscador (filtra los guardados al vuelo).
    Query(String),
    /// Buscar el texto en Telegram (Enter o el botón).
    Search,
    /// Página siguiente de la búsqueda.
    More,
    Results { query: String, offset: String, result: BackendResult<GifPage> },
    /// Reintentar cargar los guardados.
    Retry,
    /// Enviar el GIF con ese id (de lo que se ve en el panel).
    Send(i64),
    /// Terminó la carga de la animación de un documento (`None`: falló).
    AnimLoaded(i64, Option<Arc<GifFrames>>),
    /// La rejilla se desplazó: `y` = desplazamiento, `h` = alto visible (solo se animan las filas a la vista).
    Scrolled { y: f32, h: f32 },
}

/// Fotogramas de un GIF listos para dibujar.
#[derive(Debug)]
pub struct GifFrames {
    pub frames: Vec<(Handle, u32)>,
    pub total_ms: u32,
    pub bytes: usize,
}

impl GifFrames {
    /// Fotograma del instante `ms` (en bucle).
    pub fn at(&self, ms: f32) -> Option<&Handle> {
        if self.frames.is_empty() {
            return None;
        }
        let mut t = (ms.max(0.0) as u64 % u64::from(self.total_ms.max(1))) as u32;
        for (h, d) in &self.frames {
            if t < *d {
                return Some(h);
            }
            t -= d;
        }
        self.frames.last().map(|(h, _)| h)
    }
}

impl GifFrames {
    /// Fotogramas RGBA `(ancho, alto, píxeles, ms)` a `Handle`s.
    fn from_rgba(frames: impl IntoIterator<Item = (u32, u32, Vec<u8>, u32)>) -> Option<Self> {
        let (mut total_ms, mut bytes) = (0u32, 0usize);
        let frames: Vec<(Handle, u32)> = frames
            .into_iter()
            .map(|(w, h, px, ms)| {
                total_ms = total_ms.saturating_add(ms);
                bytes += px.len();
                (Handle::from_rgba(w, h, px), ms)
            })
            .collect();
        (!frames.is_empty()).then_some(GifFrames { frames, total_ms, bytes })
    }
}

/// Decodifica un GIF (MP4/H.264 o GIF real, según los bytes) en el pool bloqueante. `None` si no
/// se puede (se queda la miniatura).
pub fn decode(bytes: &[u8]) -> Option<GifFrames> {
    if bytes.starts_with(b"GIF8") {
        decode_gif(bytes)
    } else if tc_video::looks_like_mp4(bytes) {
        match tc_video::decode_mp4(bytes, &LIMITS) {
            Ok(clip) => GifFrames::from_rgba(clip.frames.into_iter().map(|f| (f.width, f.height, f.rgba, f.ms))),
            Err(e) => {
                tracing::debug!("GIF MP4 sin animar: {e}");
                None
            }
        }
    } else {
        None
    }
}

/// GIF real con `image`: fotogramas a ≤ `ANIM_PX` y ≤ `MAX_FRAMES`, luego ajustados a `LIMITS`.
fn decode_gif(bytes: &[u8]) -> Option<GifFrames> {
    let dec = GifDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    let mut frames: Vec<(RgbaImage, u32)> = Vec::new();
    for f in dec.into_frames().take(MAX_FRAMES) {
        let Ok(f) = f else { break };
        let (n, d) = f.delay().numer_denom_ms();
        let ms = if d == 0 || n == 0 { DEFAULT_DELAY_MS } else { (n / d).max(MIN_DELAY_MS) };
        let img = f.into_buffer();
        let (w, h) = img.dimensions();
        let (nw, nh) = plan::fit(w, h, ANIM_PX);
        let img = if (nw, nh) == (w, h) { img } else { ::image::imageops::resize(&img, nw, nh, FilterType::Triangle) };
        frames.push((img, ms));
    }
    let (w, h) = frames.first()?.0.dimensions();
    let durations: Vec<u32> = frames.iter().map(|f| f.1).collect();
    let p = plan::plan(w, h, &durations, &Limits { max_side: w.max(h), ..LIMITS });
    let picks = plan::pick(frames.len(), p.frames);
    let ms = plan::group_ms(&durations, &picks);
    let mut frames: Vec<Option<RgbaImage>> = frames.into_iter().map(|f| Some(f.0)).collect();
    GifFrames::from_rgba(picks.iter().zip(ms).filter_map(|(&k, ms)| {
        let img = frames.get_mut(k)?.take()?;
        let img = if (p.width, p.height) == img.dimensions() { img } else { ::image::imageops::resize(&img, p.width, p.height, FilterType::Triangle) };
        let (w, h) = img.dimensions();
        Some((w, h, img.into_raw(), ms))
    }))
}

#[derive(Debug)]
enum Slot {
    Loading,
    Ready { frames: Arc<GifFrames>, used: Cell<u64> },
    Failed(Instant),
}

/// Animaciones decodificadas por documento (LRU con tope de bytes) y cargas en curso.
#[derive(Debug, Default)]
pub struct GifAnims {
    slots: HashMap<i64, Slot>,
    clock: Cell<u64>,
    bytes: usize,
    inflight: usize,
}

impl GifAnims {
    pub fn wants(&self, file: i64, now: Instant) -> bool {
        match self.slots.get(&file) {
            None => true,
            Some(Slot::Failed(at)) => now.saturating_duration_since(*at) >= RETRY_AFTER,
            Some(_) => false,
        }
    }

    pub fn can_start(&self) -> bool {
        self.inflight < MAX_INFLIGHT
    }

    pub fn start(&mut self, file: i64) {
        if !matches!(self.slots.insert(file, Slot::Loading), Some(Slot::Loading)) {
            self.inflight += 1;
        }
    }

    pub fn finish(&mut self, file: i64, frames: Option<Arc<GifFrames>>, now: Instant) {
        if matches!(self.slots.get(&file), Some(Slot::Loading)) {
            self.inflight = self.inflight.saturating_sub(1);
        }
        if let Some(Slot::Ready { frames, .. }) = self.slots.remove(&file) {
            self.bytes -= frames.bytes;
        }
        match frames {
            Some(f) => {
                self.bytes += f.bytes;
                self.clock.set(self.clock.get() + 1);
                self.slots.insert(file, Slot::Ready { frames: f, used: Cell::new(self.clock.get()) });
                self.evict(file);
            }
            None => {
                self.slots.insert(file, Slot::Failed(now));
            }
        }
    }

    fn evict(&mut self, keep: i64) {
        while self.bytes > CACHE_BYTES {
            let oldest = self
                .slots
                .iter()
                .filter(|(k, _)| **k != keep)
                .filter_map(|(k, s)| match s {
                    Slot::Ready { used, .. } => Some((*k, used.get())),
                    _ => None,
                })
                .min_by_key(|(_, u)| *u);
            let Some((k, _)) = oldest else { break };
            if let Some(Slot::Ready { frames, .. }) = self.slots.remove(&k) {
                self.bytes -= frames.bytes;
            }
        }
    }

    /// Animación lista de un documento (y la marca como usada).
    pub fn get(&self, file: i64) -> Option<&Arc<GifFrames>> {
        match self.slots.get(&file) {
            Some(Slot::Ready { frames, used }) => {
                self.clock.set(self.clock.get() + 1);
                used.set(self.clock.get());
                Some(frames)
            }
            _ => None,
        }
    }

    pub fn is_ready(&self, file: i64) -> bool {
        matches!(self.slots.get(&file), Some(Slot::Ready { .. }))
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }
}

/// Resultado de la búsqueda en la red para una consulta.
#[derive(Debug, Clone, Default)]
pub struct Search {
    pub query: String,
    pub gifs: Vec<Gif>,
    pub next: Option<String>,
    pub loading: bool,
    pub error: Option<String>,
}

/// Estado de la pestaña de GIFs.
#[derive(Debug, Default)]
pub struct GifPanel {
    /// `None`: aún no se han pedido.
    pub saved: Option<Vec<Gif>>,
    pub saved_loading: bool,
    pub saved_error: Option<String>,
    pub query: String,
    pub search: Option<Search>,
    /// Cursor de teclado sobre `shown()` (las pestañas de `Picker` no se usan).
    pub cursor: Picker,
    /// Columnas de la rejilla en la última vista.
    pub cols: Cell<usize>,
    /// GIFs enviados aún sin confirmar (para reintentar).
    pub pending: HashMap<RandomId, Gif>,
    pub anims: GifAnims,
    /// Desplazamiento y alto visible de la rejilla (`GifMsg::Scrolled`; alto 0 = aún no se sabe).
    pub scroll_y: f32,
    pub view_h: f32,
}

impl GifPanel {
    /// GIFs guardados que casan con el texto del buscador.
    pub fn saved_matches(&self) -> Vec<&Gif> {
        let q = fold(self.query.trim());
        self.saved.iter().flatten().filter(|g| q.is_empty() || fold(&g.title).contains(&q)).collect()
    }

    /// Resultados de la red para el texto actual (si se buscó).
    pub fn results(&self) -> Option<&Search> {
        self.search.as_ref().filter(|s| s.query == self.query.trim())
    }

    /// Lo que se ve en la rejilla, en orden: guardados que casan y, después, resultados de la red.
    pub fn shown(&self) -> Vec<&Gif> {
        let mut v = self.saved_matches();
        if let Some(s) = self.results() {
            for g in &s.gifs {
                if !v.iter().any(|x| x.id == g.id) {
                    v.push(g);
                }
            }
        }
        v
    }

    pub fn find(&self, id: i64) -> Option<&Gif> {
        self.shown().into_iter().find(|g| g.id == id)
    }

    /// Índices de `shown()` (de `n`) en las filas a la vista de la rejilla, con una fila de margen.
    pub fn visible_range(&self, n: usize) -> Range<usize> {
        let cols = self.cols.get().max(1);
        let row = CELL_H + CELL_GAP;
        let h = if self.view_h > 0.0 { self.view_h } else { DEFAULT_VIEW_H };
        let first = ((self.scroll_y.max(0.0) / row).floor() as usize).saturating_sub(1);
        let last = ((self.scroll_y.max(0.0) + h) / row).ceil() as usize + 1;
        (first * cols).min(n)..(last * cols).min(n)
    }
}

/// ¿El medio de un mensaje se anima en la burbuja? Todos los GIF (`documentAttributeAnimated`):
/// MP4/H.264 (lo normal en Telegram) o GIF real; qué decodificador usar lo deciden los bytes.
pub fn animates(md: &Media) -> bool {
    md.kind == MediaKind::Gif
}

/// Nombre de archivo con el que se muestra un GIF enviado (decide si la burbuja lo anima).
pub fn file_name(g: &Gif) -> String {
    let base = if g.title.trim().is_empty() { "gif".to_string() } else { g.title.trim().replace(' ', "_") };
    let lower = base.to_ascii_lowercase();
    if lower.ends_with(".gif") || lower.ends_with(".mp4") {
        base
    } else {
        format!("{base}.{}", if g.is_real_gif() { "gif" } else { "mp4" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::codecs::gif::{GifEncoder, Repeat};
    use ::image::{Delay, Frame, Rgba, RgbaImage};

    pub(crate) fn sample_gif(w: u32, h: u32, frames: u32, ms: u32) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = GifEncoder::new(&mut out);
            enc.set_repeat(Repeat::Infinite).unwrap();
            let fs = (0..frames).map(|k| Frame::from_parts(RgbaImage::from_pixel(w, h, Rgba([(k * 40) as u8, 10, 10, 255])), 0, 0, Delay::from_numer_denom_ms(ms, 1)));
            enc.encode_frames(fs).unwrap();
        }
        out
    }

    #[test]
    fn decodifica_escala_y_recorre_los_fotogramas() {
        let f = decode(&sample_gif(640, 320, 4, 50)).expect("GIF válido");
        assert_eq!(f.frames.len(), 4);
        assert_eq!(f.total_ms, 200);
        assert_eq!(f.bytes, 4 * (320 * 160 * 4) as usize, "escalado a ANIM_PX");
        let id = |h: Option<&Handle>| h.map(Handle::id);
        assert_eq!(id(f.at(0.0)), Some(f.frames[0].0.id()));
        assert_eq!(id(f.at(60.0)), Some(f.frames[1].0.id()));
        assert_eq!(id(f.at(260.0)), Some(f.frames[1].0.id()), "en bucle");
        assert!(decode(b"no es un gif").is_none());
        // Retardo 0 → 100 ms; retardos ínfimos → MIN_DELAY_MS (el GIF guarda centésimas).
        assert_eq!(decode(&sample_gif(8, 8, 2, 0)).unwrap().total_ms, 2 * DEFAULT_DELAY_MS);
        assert_eq!(decode(&sample_gif(8, 8, 2, 10)).unwrap().total_ms, 2 * MIN_DELAY_MS, "10 ms (1 cs) se sube al mínimo");
    }

    #[test]
    fn decodifica_los_mp4_de_telegram() {
        // MP4/H.264 (perfil High con fotogramas B) de 160×120, 12 fotogramas a 80 ms.
        let f = decode(tc_mock::gifs::SAMPLE_MP4).expect("MP4 válido");
        assert_eq!(f.frames.len(), 12);
        assert_eq!(f.total_ms, 960);
        assert_eq!(f.bytes, 12 * 160 * 120 * 4);
        assert_ne!(f.at(0.0).map(Handle::id), f.at(500.0).map(Handle::id));
        // Lo que no es ni GIF ni MP4, o un MP4 roto: sin animación (se queda la miniatura).
        assert!(decode(&[0u8; 64]).is_none());
        assert!(decode(&tc_mock::gifs::SAMPLE_MP4[..200]).is_none());
    }

    #[test]
    fn un_gif_grande_se_ajusta_al_presupuesto() {
        let f = decode(&sample_gif(320, 320, 40, 50)).expect("GIF válido");
        assert_eq!(f.frames.len(), 40, "basta con encoger");
        assert!(f.bytes <= ANIM_BUDGET && f.bytes > ANIM_BUDGET / 2, "{} de {ANIM_BUDGET}", f.bytes);
        assert_eq!(f.total_ms, 40 * 50);
    }

    #[test]
    fn solo_se_animan_las_filas_a_la_vista() {
        let p = GifPanel::default();
        p.cols.set(3);
        // Sin medidas: 4 filas supuestas + 1 de margen.
        assert_eq!(p.visible_range(100), 0..15);
        let p = GifPanel { scroll_y: 10.0 * (CELL_H + CELL_GAP), view_h: 2.0 * (CELL_H + CELL_GAP), ..Default::default() };
        p.cols.set(3);
        assert_eq!(p.visible_range(100), 27..39);
        assert_eq!(p.visible_range(30), 27..30);
        assert_eq!(p.visible_range(5), 5..5);
    }

    #[test]
    fn la_cache_de_animaciones_respeta_el_tope() {
        let mut a = GifAnims::default();
        let now = Instant::now();
        let big = |n: usize| Arc::new(GifFrames { frames: vec![(Handle::from_rgba(1, 1, vec![0; 4]), 10)], total_ms: 10, bytes: n });
        a.start(1);
        a.start(2);
        assert!(!a.wants(1, now));
        a.finish(1, Some(big(CACHE_BYTES / 2)), now);
        a.finish(2, Some(big(CACHE_BYTES / 2)), now);
        let _ = a.get(1); // 1 usado después que 2
        a.start(3);
        a.finish(3, Some(big(CACHE_BYTES / 2)), now);
        assert!(a.is_ready(1) && !a.is_ready(2) && a.is_ready(3));
        assert!(a.bytes() <= CACHE_BYTES);
        a.start(4);
        a.finish(4, None, now);
        assert!(!a.wants(4, now) && a.wants(4, now + RETRY_AFTER));
        assert!(a.can_start());
    }

    fn g(id: i64, title: &str) -> Gif {
        Gif { id, file: None, mime: "video/mp4".into(), width: 1, height: 1, duration_secs: None, title: title.into(), inline: None }
    }

    #[test]
    fn lo_que_se_ve_filtra_guardados_y_anade_resultados() {
        let mut p = GifPanel { saved: Some(vec![g(1, "Gato bailando"), g(2, "olas")]), ..Default::default() };
        assert_eq!(p.shown().len(), 2);
        p.query = "gato".into();
        assert_eq!(p.shown().iter().map(|g| g.id).collect::<Vec<_>>(), [1]);
        p.search = Some(Search { query: "gato".into(), gifs: vec![g(1, "x"), g(9, "gato")], ..Default::default() });
        assert_eq!(p.shown().iter().map(|g| g.id).collect::<Vec<_>>(), [1, 9], "sin repetir");
        p.query = "olas".into();
        assert!(p.results().is_none(), "los resultados son de otra consulta");
        assert_eq!(file_name(&g(1, "gato bailando")), "gato_bailando.mp4");
        assert_eq!(file_name(&Gif { mime: tc_core::GIF_MIME.into(), ..g(1, "") }), "gif.gif");
    }
}
