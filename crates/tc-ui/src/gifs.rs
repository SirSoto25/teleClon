//! GIFs en la UI (ADR-0010): pestaña del panel (guardados, búsqueda en lo guardado y en la red) y
//! animación de los GIF reales.
//!
//! * Los GIF de Telegram son casi siempre MP4/H.264 sin sonido: no hay decodificador de vídeo
//!   portable en el árbol, así que se dibuja su miniatura estática con la insignia «GIF».
//! * Los GIF de verdad (`image/gif`, o un `.gif` en la burbuja) se descargan enteros (como mucho
//!   `MAX_ANIM_BYTES`), se decodifican con `image` fuera del hilo de la UI a fotogramas de como
//!   mucho `ANIM_PX` y se animan con el reloj compartido de los stickers (`App::sticker_ms`, reloj
//!   de `sticker_anim.rs`); con movimiento reducido, el primer fotograma.

use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ::image::AnimationDecoder;
use ::image::codecs::gif::GifDecoder;
use ::image::imageops::FilterType;
use iced::widget::image::Handle;
use tc_core::{BackendResult, Gif, GifPage, Media, MediaKind, RandomId};

use crate::format::fold;
use crate::stickers::Picker;

/// GIFs más grandes que esto no se animan (se queda la miniatura).
pub const MAX_ANIM_BYTES: u64 = 8 * 1024 * 1024;
/// Lado máximo (px físicos) de los fotogramas animados.
pub const ANIM_PX: u32 = 320;
/// Fotogramas como mucho por GIF (los demás se descartan).
pub const MAX_FRAMES: usize = 150;
/// Tope de memoria de todos los fotogramas decodificados.
pub const CACHE_BYTES: usize = 64 * 1024 * 1024;
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

/// Decodifica un GIF animado (en el pool bloqueante). `None` si no es un GIF válido.
pub fn decode(bytes: &[u8]) -> Option<GifFrames> {
    let dec = GifDecoder::new(std::io::Cursor::new(bytes)).ok()?;
    let mut frames = Vec::new();
    let (mut total_ms, mut total_bytes) = (0u32, 0usize);
    for f in dec.into_frames().take(MAX_FRAMES) {
        let Ok(f) = f else { break };
        let (n, d) = f.delay().numer_denom_ms();
        let ms = if d == 0 || n == 0 { DEFAULT_DELAY_MS } else { (n / d).max(MIN_DELAY_MS) };
        let img = f.into_buffer();
        let (w, h) = img.dimensions();
        let img = if w.max(h) > ANIM_PX {
            let (nw, nh) = if w >= h { (ANIM_PX, (h * ANIM_PX / w).max(1)) } else { ((w * ANIM_PX / h).max(1), ANIM_PX) };
            ::image::imageops::resize(&img, nw, nh, FilterType::Triangle)
        } else {
            img
        };
        let (w, h) = img.dimensions();
        total_bytes += (w * h * 4) as usize;
        total_ms += ms;
        frames.push((Handle::from_rgba(w, h, img.into_raw()), ms));
    }
    (!frames.is_empty()).then_some(GifFrames { frames, total_ms, bytes: total_bytes })
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
}

/// ¿El medio de un mensaje es un GIF de verdad (animable con `image`)?
pub fn is_real_gif_media(md: &Media) -> bool {
    md.kind == MediaKind::Gif && md.file_name.as_deref().is_some_and(|n| n.to_ascii_lowercase().ends_with(".gif"))
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
