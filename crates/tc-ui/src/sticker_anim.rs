//! Stickers TGS animados en la UI (ADR-0008).
//!
//! * Un único reloj a 30 fps como mucho (`TICK`, `iced::time::every`), suscrito solo si hay algo
//!   animado a la vista, la ventana tiene el foco y el movimiento no está reducido.
//! * En cada tic, para cada TGS visible (burbujas de la ventana visible y, en el selector, el sticker
//!   bajo el ratón o el del cursor de teclado): si falta el TGS se descarga y se prepara
//!   (`Tgs::load`); si falta el fotograma que toca, se pinta un lote (ese y los siguientes) en el pool
//!   bloqueante. Mientras llega, se mantiene el último fotograma mostrado.
//! * Fotogramas a `ANIM_PX` en una LRU acotada en bytes (`FRAME_CACHE_BYTES`), como `Handle` de
//!   iced (la GPU no vuelve a subir un fotograma que ya tiene); los TGS preparados, en otra LRU.
//! * Movimiento reducido: nada de esto se usa y se dibuja la imagen estática (primer fotograma o
//!   miniatura, ver `sticker_img.rs`). Sin foco: congelado en el último fotograma.
//! * WebM (vídeo VP9 con alfa): sigue estático, ver el ADR.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::image::Handle;
use tc_core::{Sticker, StickerFormat, StickerId};
use tc_stickers::anim::{self, ANIM_PX, Playback, Show, Tgs};
use tc_stickers::{ByteLru, CacheKey};

use crate::app::Msg;

/// Periodo del reloj de animación (30 fps).
pub const TICK: Duration = Duration::from_micros(33_334);
/// Tope de la caché de fotogramas pintados (48 MiB ≈ 190 fotogramas de 256 px).
pub const FRAME_CACHE_BYTES: usize = 48 * 1024 * 1024;
/// Tope de los TGS ya descomprimidos (JSON Lottie).
pub const DATA_CACHE_BYTES: usize = 16 * 1024 * 1024;
/// Fotogramas que se pintan por adelantado en cada lote (además del que toca).
pub const LOOKAHEAD: usize = 6;
/// Lotes de render en curso a la vez (uno por sticker como mucho).
pub const MAX_RENDERS: usize = 3;
/// Espera antes de reintentar un TGS que no se pudo descargar o preparar.
pub const RETRY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub enum AnimMsg {
    /// Tic del reloj de animación.
    Tick(Instant),
    /// TGS descargado y preparado (`None`: falló la descarga o no es un Lottie válido).
    Loaded(i64, Option<Arc<Tgs>>),
    /// Lote de fotogramas pintados de un documento (los que fallaron no vienen).
    Frames(i64, Vec<(u32, Handle)>),
    /// La ventana gana o pierde el foco.
    Focus(bool),
    /// El ratón entra o sale de una celda del selector.
    Hover(StickerId),
    Unhover(StickerId),
}

/// ¿Este sticker se anima con rlottie en esta compilación? (TGS de la red con archivo).
pub fn plays(s: &Sticker) -> bool {
    tc_stickers::tgs::lottie_enabled() && s.format == StickerFormat::Lottie && s.file.is_some()
}

/// Foco de la ventana como mensaje (para `iced::event::listen_with`).
pub fn focus_event(event: iced::Event, _status: iced::event::Status, _window: iced::window::Id) -> Option<Msg> {
    match event {
        iced::Event::Window(iced::window::Event::Focused) => Some(Msg::StickerAnim(AnimMsg::Focus(true))),
        iced::Event::Window(iced::window::Event::Unfocused) => Some(Msg::StickerAnim(AnimMsg::Focus(false))),
        _ => None,
    }
}

/// Trabajo que la app convierte en `Task`.
#[derive(Debug)]
pub enum Job {
    /// Descargar el TGS de este documento y prepararlo.
    Load(i64),
    /// Pintar estos fotogramas a `ANIM_PX`.
    Render { file: i64, tgs: Arc<Tgs>, frames: Vec<usize> },
}

fn key(file: i64, frame: usize) -> CacheKey {
    CacheKey { sticker_id: file, size: ANIM_PX, frame: frame as u32 }
}

/// Estado de las animaciones TGS.
#[derive(Debug)]
pub struct StickerAnims {
    data: ByteLru<i64, Arc<Tgs>>,
    loading: HashSet<i64>,
    failed: HashMap<i64, Instant>,
    frames: ByteLru<CacheKey, Handle>,
    rendering: HashSet<i64>,
    /// Fotograma que se muestra ahora por documento (solo de los activos).
    current: HashMap<i64, Handle>,
    /// La ventana tiene el foco (sin él, todo queda congelado).
    pub focused: bool,
    /// Sticker del selector bajo el ratón.
    pub hovered: Option<StickerId>,
}

impl Default for StickerAnims {
    fn default() -> Self {
        Self::with_limits(DATA_CACHE_BYTES, FRAME_CACHE_BYTES)
    }
}

impl StickerAnims {
    pub fn with_limits(data_bytes: usize, frame_bytes: usize) -> Self {
        Self {
            data: ByteLru::new(data_bytes),
            loading: HashSet::new(),
            failed: HashMap::new(),
            frames: ByteLru::new(frame_bytes),
            rendering: HashSet::new(),
            current: HashMap::new(),
            focused: true,
            hovered: None,
        }
    }

    /// Política con la preferencia de movimiento de la app.
    pub fn playback(&self, reduced_motion: bool) -> Playback {
        Playback { reduced_motion, focused: self.focused }
    }

    /// Avanza las animaciones de los documentos `active` (visibles) al instante `elapsed` del reloj
    /// y devuelve lo que hay que cargar o pintar.
    pub fn plan(&mut self, playback: Playback, elapsed: Duration, active: &[i64], now: Instant) -> Vec<Job> {
        self.current.retain(|f, _| active.contains(f));
        let mut jobs = Vec::new();
        if !playback.ticks(!active.is_empty()) {
            return jobs;
        }
        for &file in active {
            let Some(tgs) = self.data.get(&file).cloned() else {
                let retry = self.failed.get(&file).is_none_or(|t| now.saturating_duration_since(*t) >= RETRY);
                if retry && self.loading.insert(file) {
                    jobs.push(Job::Load(file));
                }
                continue;
            };
            let Show::Frame(f) = playback.show(&tgs.info, true, elapsed) else { continue };
            if let Some(h) = self.frames.get(&key(file, f)) {
                self.current.insert(file, h.clone());
            }
            if self.rendering.contains(&file) || self.rendering.len() >= MAX_RENDERS {
                continue;
            }
            let frames: Vec<usize> = std::iter::once(f)
                .chain(anim::next_frames(&tgs.info, f, LOOKAHEAD))
                .filter(|g| !self.frames.contains(&key(file, *g)))
                .collect();
            if !frames.is_empty() {
                self.rendering.insert(file);
                jobs.push(Job::Render { file, tgs, frames });
            }
        }
        jobs
    }

    /// Llega un TGS preparado (o el fallo de su carga).
    pub fn loaded(&mut self, file: i64, tgs: Option<Arc<Tgs>>, now: Instant) {
        self.loading.remove(&file);
        match tgs {
            Some(t) => {
                let bytes = t.byte_len();
                self.failed.remove(&file);
                self.data.insert(file, t, bytes);
            }
            None => {
                self.failed.insert(file, now);
            }
        }
    }

    /// Llega un lote de fotogramas. Un lote vacío (rlottie no pudo) cuenta como fallo del TGS.
    pub fn rendered(&mut self, file: i64, frames: Vec<(u32, Handle)>, now: Instant) {
        self.rendering.remove(&file);
        if frames.is_empty() {
            self.data.remove(&file);
            self.failed.insert(file, now);
            return;
        }
        let bytes = ANIM_PX as usize * ANIM_PX as usize * 4;
        for (f, h) in frames {
            self.frames.insert(key(file, f as usize), h, bytes);
        }
    }

    /// Fotograma animado que se dibuja para `file`, o `None` para usar la imagen estática.
    pub fn frame(&self, file: i64, playback: Playback) -> Option<&Handle> {
        if playback.reduced_motion {
            return None;
        }
        self.current.get(&file)
    }

    /// Bytes de fotogramas en caché (y su tope).
    pub fn frame_bytes(&self) -> (usize, usize) {
        (self.frames.used_bytes(), self.frames.max_bytes())
    }

    pub fn has_data(&self, file: i64) -> bool {
        self.data.contains(&file)
    }
}

/// Ejecuta `f` en el pool bloqueante de tokio (o en un hilo propio si no hay runtime).
pub(crate) async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.spawn_blocking(f).await.ok(),
        Err(_) => {
            let (tx, rx) = futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(f());
            });
            rx.await.ok()
        }
    }
}

/// Pinta un lote fuera del hilo de la UI y lo convierte en `Handle`s.
pub(crate) fn render_handles(tgs: &Tgs, frames: &[usize]) -> Vec<(u32, Handle)> {
    tgs.render_many(frames, ANIM_PX)
        .into_iter()
        .filter_map(|(f, r)| r.ok().map(|fr| (f as u32, Handle::from_rgba(fr.width, fr.height, fr.rgba))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lottie mínimo animado (cuadrado que se desplaza; 60 fotogramas a 60 fps).
    const MOVING: &str = r#"{"v":"5.5.2","fr":60,"ip":0,"op":60,"w":64,"h":64,"nm":"m","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},"p":{"a":1,"k":[{"t":0,"s":[16,32,0],"e":[48,32,0],"i":{"x":[1],"y":[1]},"o":{"x":[0],"y":[0]}},{"t":59}]},"a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},"ao":0,"shapes":[{"ty":"gr","it":[{"ty":"rc","d":1,"s":{"a":0,"k":[20,20]},"p":{"a":0,"k":[0,0]},"r":{"a":0,"k":0}},{"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},{"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}]}],"ip":0,"op":60,"st":0,"bm":0}]}"#;

    fn tgs() -> Option<Arc<Tgs>> {
        Tgs::load(&tc_stickers::tgs::compress(MOVING.as_bytes()), "ui-anim-test").ok().map(Arc::new)
    }

    const FULL: Playback = Playback { reduced_motion: false, focused: true };

    #[test]
    fn carga_una_vez_y_reintenta_tras_fallo() {
        let mut a = StickerAnims::default();
        let t0 = Instant::now();
        let jobs = a.plan(FULL, Duration::ZERO, &[7], t0);
        assert!(matches!(jobs.as_slice(), [Job::Load(7)]));
        assert!(a.plan(FULL, Duration::ZERO, &[7], t0).is_empty(), "no se pide dos veces");
        a.loaded(7, None, t0);
        assert!(a.plan(FULL, Duration::ZERO, &[7], t0 + Duration::from_secs(1)).is_empty(), "espera antes de reintentar");
        assert!(matches!(a.plan(FULL, Duration::ZERO, &[7], t0 + RETRY).as_slice(), [Job::Load(7)]));
    }

    #[test]
    fn sin_foco_o_con_movimiento_reducido_no_hay_trabajo() {
        let mut a = StickerAnims::default();
        let t0 = Instant::now();
        let reduced = Playback { reduced_motion: true, focused: true };
        assert!(a.plan(reduced, Duration::ZERO, &[7], t0).is_empty());
        let unfocused = Playback { reduced_motion: false, focused: false };
        assert!(a.plan(unfocused, Duration::ZERO, &[7], t0).is_empty());
        assert!(a.plan(FULL, Duration::ZERO, &[], t0).is_empty(), "nada visible");
    }

    #[test]
    fn pinta_por_lotes_muestra_y_congela() {
        let Some(t) = tgs() else {
            assert!(!tc_stickers::tgs::lottie_enabled());
            return;
        };
        let mut a = StickerAnims::default();
        let now = Instant::now();
        a.loaded(7, Some(t.clone()), now);
        assert!(a.has_data(7));
        let jobs = a.plan(FULL, Duration::ZERO, &[7], now);
        let [Job::Render { file: 7, frames, tgs }] = jobs.as_slice() else { panic!("lote esperado: {jobs:?}") };
        assert_eq!(frames, &[0, 2, 4, 6, 8, 10, 12], "el que toca y los siguientes a 30 fps");
        assert!(a.plan(FULL, Duration::ZERO, &[7], now).is_empty(), "un lote por sticker a la vez");
        assert!(a.frame(7, FULL).is_none(), "aún nada que mostrar: imagen estática");

        let out = render_handles(tgs, frames);
        assert_eq!(out.len(), 7);
        a.rendered(7, out, now);
        // A los 100 ms toca el 6 (ya pintado): se muestra y se adelantan solo los que faltan.
        let jobs = a.plan(FULL, Duration::from_millis(100), &[7], now);
        assert!(matches!(jobs.as_slice(), [Job::Render { frames, .. }] if frames == &[14, 16, 18]), "{jobs:?}");
        let shown = a.frame(7, FULL).cloned().expect("fotograma animado");
        assert!(a.frame(7, Playback { reduced_motion: true, focused: true }).is_none(), "movimiento reducido: estático");
        // Sin foco: congelado en el mismo fotograma, sin trabajo nuevo.
        let unfocused = Playback { reduced_motion: false, focused: false };
        assert!(a.plan(unfocused, Duration::from_secs(1), &[7], now).is_empty());
        assert_eq!(a.frame(7, unfocused).map(Handle::id), Some(shown.id()));
        // Fuera de la vista: se olvida el fotograma actual.
        let _ = a.plan(FULL, Duration::from_secs(1), &[], now);
        assert!(a.frame(7, FULL).is_none());
        // Un lote vacío (rlottie falló) descarta el TGS hasta el reintento.
        let _ = a.plan(FULL, Duration::from_secs(1), &[7], now);
        a.rendered(7, Vec::new(), now);
        assert!(!a.has_data(7));
    }

    #[test]
    fn la_cache_de_fotogramas_respeta_su_tope() {
        let Some(t) = tgs() else { return };
        let frame = ANIM_PX as usize * ANIM_PX as usize * 4;
        let mut a = StickerAnims::with_limits(DATA_CACHE_BYTES, frame * 4);
        let now = Instant::now();
        a.loaded(1, Some(t.clone()), now);
        let all: Vec<usize> = (0..12).collect();
        a.rendered(1, render_handles(&t, &all), now);
        let (used, max) = a.frame_bytes();
        assert_eq!((used, max), (frame * 4, frame * 4), "solo caben 4 fotogramas");
        // Los que quedan son los más recientes (8..12): pedir el 0 vuelve a pintarlo.
        assert!(matches!(a.plan(FULL, Duration::ZERO, &[1], now).as_slice(), [Job::Render { frames, .. }] if frames[0] == 0));
    }
}
