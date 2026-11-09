//! Fotos en la UI: avatares y fotos de los mensajes, cargados de forma perezosa y asíncrona.
//!
//! * La app decide qué se ve (filas visibles de la lista, cabecera, ventana visible de la
//!   conversación, visor) y pide solo eso, como mucho `MAX_INFLIGHT` a la vez; al terminar una carga
//!   se vuelve a mirar qué falta.
//! * La descarga va por `MediaApi` (con la caché de disco de tc-store si el backend la tiene) y el
//!   decodificado (`image`: JPEG/PNG/WebP), recorte y escalado, en el pool bloqueante de tokio: nunca
//!   en el hilo de la UI.
//! * Los `Handle` decodificados se guardan en una caché en memoria con tope de bytes (LRU). Mientras
//!   carga, o si falla, se dibuja el avatar generado de Brasa: no hay parpadeo porque el `Handle` de
//!   cada foto se crea una vez y se reutiliza.

use std::cell::Cell;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use ::image::imageops::FilterType;
use ::image::{DynamicImage, GenericImageView};
use iced::widget::image::Handle;
use tc_core::{FileRef, MediaVariant, MsgId, PeerRef, PhotoSize, ProfilePhoto};

use crate::app::Backend;

/// Lado (px físicos) al que se decodifican los avatares: 2× el mayor de la lista (44 px).
pub const AVATAR_PX: u32 = 96;
/// Ancho (px físicos) de la foto en la burbuja: 2× el ancho máximo del medio (320 px).
pub const BUBBLE_PX: u32 = 640;
/// Lado máximo de la foto en el visor.
pub const VIEWER_PX: u32 = 2560;
/// Cargas simultáneas desde la UI (el backend limita además las descargas de red).
pub const MAX_INFLIGHT: usize = 6;
/// Tope de la caché en memoria (RGBA decodificado).
pub const CACHE_BYTES: usize = 96 * 1024 * 1024;
/// Tras un fallo no se reintenta hasta pasado este tiempo.
pub const RETRY_AFTER: Duration = Duration::from_secs(30);
/// Proporción alto/ancho del medio en la burbuja: la misma horquilla que `conv::layout_message`
/// (alto entre 120 y 360 px para 320 px de ancho).
pub const BUBBLE_ASPECT: (f32, f32) = (120.0 / 320.0, 360.0 / 320.0);

/// Identidad de una imagen en pantalla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageKey {
    /// Foto de perfil (`photo` cambia cuando cambia la foto).
    Avatar { peer: PeerRef, photo: i64 },
    /// Vista previa de un medio en su burbuja.
    Thumb { peer: PeerRef, msg: MsgId, file: i64 },
    /// Foto completa para el visor.
    Full { peer: PeerRef, msg: MsgId, file: i64 },
}

/// Qué cargar para una clave.
#[derive(Debug, Clone)]
pub enum ImageRequest {
    Avatar { peer: PeerRef, photo: ProfilePhoto },
    Media { peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant, aspect: Option<f32> },
}

impl ImageRequest {
    pub fn key(&self) -> ImageKey {
        match self {
            Self::Avatar { peer, photo } => ImageKey::Avatar { peer: *peer, photo: photo.id },
            Self::Media { peer, msg, file, variant: MediaVariant::Thumb, .. } => ImageKey::Thumb { peer: *peer, msg: *msg, file: file.id },
            Self::Media { peer, msg, file, variant: MediaVariant::Full, .. } => ImageKey::Full { peer: *peer, msg: *msg, file: file.id },
        }
    }
}

/// Imagen lista para dibujar.
#[derive(Debug, Clone)]
pub struct Decoded {
    pub handle: Handle,
    pub width: u32,
    pub height: u32,
}

impl Decoded {
    fn bytes(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    fn from_image(img: DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self { handle: Handle::from_rgba(width, height, rgba.into_raw()), width, height }
    }
}

#[derive(Debug)]
enum Slot {
    Loading,
    Ready { img: Decoded, used: Cell<u64> },
    Failed(Instant),
}

/// Caché en memoria de imágenes decodificadas (LRU con tope de bytes) y estado de las cargas.
#[derive(Debug, Default)]
pub struct Images {
    slots: HashMap<ImageKey, Slot>,
    clock: Cell<u64>,
    bytes: usize,
    inflight: usize,
}

impl Images {
    /// Imagen lista (y la marca como usada).
    pub fn get(&self, key: &ImageKey) -> Option<&Decoded> {
        match self.slots.get(key) {
            Some(Slot::Ready { img, used }) => {
                self.clock.set(self.clock.get() + 1);
                used.set(self.clock.get());
                Some(img)
            }
            _ => None,
        }
    }

    pub fn handle(&self, key: &ImageKey) -> Option<Handle> {
        self.get(key).map(|d| d.handle.clone())
    }

    pub fn is_loading(&self, key: &ImageKey) -> bool {
        matches!(self.slots.get(key), Some(Slot::Loading))
    }

    pub fn failed(&self, key: &ImageKey) -> bool {
        matches!(self.slots.get(key), Some(Slot::Failed(_)))
    }

    /// ¿Hay que pedirla? (no está, o falló hace tiempo).
    pub fn wants(&self, key: &ImageKey, now: Instant) -> bool {
        match self.slots.get(key) {
            None => true,
            Some(Slot::Failed(at)) => now.saturating_duration_since(*at) >= RETRY_AFTER,
            Some(_) => false,
        }
    }

    pub fn can_start(&self) -> bool {
        self.inflight < MAX_INFLIGHT
    }

    pub fn inflight(&self) -> usize {
        self.inflight
    }

    /// Bytes decodificados en memoria.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn start(&mut self, key: ImageKey) {
        if !matches!(self.slots.insert(key, Slot::Loading), Some(Slot::Loading)) {
            self.inflight += 1;
        }
    }

    /// Resultado de una carga: guarda la imagen y desaloja las menos usadas si se pasa del tope.
    pub fn finish(&mut self, key: ImageKey, img: Option<Decoded>, now: Instant) {
        if matches!(self.slots.get(&key), Some(Slot::Loading)) {
            self.inflight = self.inflight.saturating_sub(1);
        }
        if let Some(Slot::Ready { img, .. }) = self.slots.remove(&key) {
            self.bytes -= img.bytes();
        }
        match img {
            Some(img) => {
                self.bytes += img.bytes();
                self.clock.set(self.clock.get() + 1);
                self.slots.insert(key, Slot::Ready { img, used: Cell::new(self.clock.get()) });
                self.evict(key);
            }
            None => {
                self.slots.insert(key, Slot::Failed(now));
            }
        }
    }

    fn evict(&mut self, keep: ImageKey) {
        while self.bytes > CACHE_BYTES {
            let oldest = self
                .slots
                .iter()
                .filter(|(k, _)| **k != keep)
                .filter_map(|(k, s)| match s {
                    Slot::Ready { used, .. } => Some((used.get(), *k)),
                    _ => None,
                })
                .min_by_key(|(u, _)| *u);
            let Some((_, k)) = oldest else { break };
            if let Some(Slot::Ready { img, .. }) = self.slots.remove(&k) {
                self.bytes -= img.bytes();
            }
        }
    }

    /// Olvida todo (cambio de cuenta). Las cargas en curso se descartan al llegar.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn len(&self) -> usize {
        self.slots.values().filter(|s| matches!(s, Slot::Ready { .. })).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ------------------------------------------------------------------ decodificado

/// Avatar: recorte cuadrado centrado y escalado a `px` (el redondeo lo pone el widget).
pub fn decode_avatar(bytes: &[u8], px: u32) -> Option<Decoded> {
    let img = ::image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();
    let side = w.min(h);
    if side == 0 {
        return None;
    }
    let sq = img.crop_imm((w - side) / 2, (h - side) / 2, side, side);
    let out = if side > px { sq.resize_exact(px, px, FilterType::Triangle) } else { sq };
    Some(Decoded::from_image(out))
}

/// Foto para la burbuja: recorte centrado a la proporción `aspect` (alto/ancho; `None` = la suya) y
/// reducida a `max_w` de ancho como mucho.
pub fn decode_cover(bytes: &[u8], aspect: Option<f32>, max_w: u32) -> Option<Decoded> {
    let img = ::image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let img = match aspect.filter(|a| a.is_finite() && *a > 0.0) {
        Some(a) => {
            let (cw, ch) = if (h as f32) > w as f32 * a { (w, ((w as f32 * a).round() as u32).clamp(1, h)) } else { (((h as f32 / a).round() as u32).clamp(1, w), h) };
            img.crop_imm((w - cw) / 2, (h - ch) / 2, cw, ch)
        }
        None => img,
    };
    let (w, h) = img.dimensions();
    let img = if w > max_w { img.resize(max_w, ((h as f32 * max_w as f32 / w as f32).round() as u32).max(1), FilterType::Triangle) } else { img };
    Some(Decoded::from_image(img))
}

/// Foto para el visor: entera, reducida para caber en `max_side`.
pub fn decode_fit(bytes: &[u8], max_side: u32) -> Option<Decoded> {
    let img = ::image::load_from_memory(bytes).ok()?;
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return None;
    }
    let img = if w.max(h) > max_side { img.resize(max_side, max_side, FilterType::Triangle) } else { img };
    Some(Decoded::from_image(img))
}

/// Proporción del medio en la burbuja (la misma horquilla que la maquetación).
pub fn bubble_aspect(width: u32, height: u32) -> Option<f32> {
    (width > 0 && height > 0).then(|| (height as f32 / width as f32).clamp(BUBBLE_ASPECT.0, BUBBLE_ASPECT.1))
}

/// Ejecuta `f` en el pool bloqueante de tokio (o en un hilo propio si no hay runtime).
async fn off_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
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

/// Descarga y decodifica (fuera del hilo de la UI). `None` si falla cualquiera de los dos pasos.
pub async fn load(backend: Backend, req: ImageRequest) -> Option<Decoded> {
    match req {
        ImageRequest::Avatar { peer, photo } => {
            let bytes = backend.download_profile_photo(peer, photo, PhotoSize::Small).await.ok()?;
            off_thread(move || decode_avatar(&bytes, AVATAR_PX)).await.flatten()
        }
        ImageRequest::Media { peer, msg, file, variant, aspect } => {
            let bytes = backend.download_media(peer, msg, file, variant).await.ok()?;
            match variant {
                MediaVariant::Thumb => off_thread(move || decode_cover(&bytes, aspect, BUBBLE_PX)).await.flatten(),
                MediaVariant::Full => off_thread(move || decode_fit(&bytes, VIEWER_PX)).await.flatten(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::image::{ImageEncoder, RgbaImage, codecs::png::PngEncoder};
    use tc_core::UserId;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let img = RgbaImage::from_fn(w, h, |x, y| ::image::Rgba([(x % 256) as u8, (y % 256) as u8, 7, 255]));
        let mut out = Vec::new();
        PngEncoder::new(&mut out).write_image(img.as_raw(), w, h, ::image::ExtendedColorType::Rgba8).unwrap();
        out
    }

    fn jpeg(w: u32, h: u32) -> Vec<u8> {
        let img = ::image::RgbImage::from_fn(w, h, |x, _| ::image::Rgb([(x % 256) as u8, 90, 40]));
        let mut out = Vec::new();
        ::image::codecs::jpeg::JpegEncoder::new(&mut out).encode_image(&img).unwrap();
        out
    }

    #[test]
    fn avatars_are_square_and_downscaled() {
        let d = decode_avatar(&png(300, 200), AVATAR_PX).unwrap();
        assert_eq!((d.width, d.height), (AVATAR_PX, AVATAR_PX));
        // Más pequeña que el destino: se queda en su tamaño (recortada a cuadrado).
        let d = decode_avatar(&jpeg(40, 60), AVATAR_PX).unwrap();
        assert_eq!((d.width, d.height), (40, 40));
        assert!(decode_avatar(b"no es una imagen", AVATAR_PX).is_none());
    }

    #[test]
    fn bubble_photos_are_cropped_to_the_layout_ratio() {
        // 1000×1000 en una caja 0.5: recorte a 1000×500 y luego 640×320.
        let d = decode_cover(&png(1000, 1000), Some(0.5), BUBBLE_PX).unwrap();
        assert_eq!((d.width, d.height), (640, 320));
        // Muy alta: la horquilla deja 1.125.
        let a = bubble_aspect(100, 1000).unwrap();
        assert!((a - BUBBLE_ASPECT.1).abs() < 1e-6);
        let d = decode_cover(&jpeg(400, 2000), Some(a), BUBBLE_PX).unwrap();
        assert_eq!((d.width, d.height), (400, 450));
        assert_eq!(bubble_aspect(0, 10), None);
        let d = decode_fit(&png(3000, 1500), 1000).unwrap();
        assert_eq!((d.width, d.height), (1000, 500));
    }

    fn img(side: u32) -> Option<Decoded> {
        Some(Decoded { handle: Handle::from_rgba(1, 1, vec![0u8; 4]), width: side, height: side })
    }

    #[test]
    fn cache_tracks_loads_failures_and_evicts_least_recently_used() {
        let mut c = Images::default();
        let now = Instant::now();
        let k = |i: i64| ImageKey::Avatar { peer: PeerRef::User(UserId(i)), photo: i };
        assert!(c.wants(&k(1), now));
        c.start(k(1));
        c.start(k(1));
        assert_eq!(c.inflight(), 1);
        assert!(!c.wants(&k(1), now) && c.is_loading(&k(1)));
        c.finish(k(1), None, now);
        assert_eq!(c.inflight(), 0);
        assert!(c.failed(&k(1)) && !c.wants(&k(1), now));
        assert!(c.wants(&k(1), now + RETRY_AFTER));

        // Cada imagen de 2048² ocupa 16 MiB: caben 6 en 96 MiB.
        for i in 10..16 {
            c.start(k(i));
            c.finish(k(i), img(2048), now);
        }
        assert_eq!(c.len(), 6);
        assert!(c.get(&k(10)).is_some()); // la 10 pasa a ser la más reciente
        c.start(k(16));
        c.finish(k(16), img(2048), now);
        assert_eq!(c.len(), 6);
        assert!(c.bytes() <= CACHE_BYTES);
        assert!(c.get(&k(11)).is_none(), "sale la de uso más antiguo");
        assert!(c.get(&k(10)).is_some() && c.get(&k(16)).is_some());
        c.clear();
        assert!(c.is_empty() && c.bytes() == 0);
    }
}
