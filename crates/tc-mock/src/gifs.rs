//! GIFs del mock: unos «guardados» de la cuenta y una búsqueda simulada (como el bot `@gif`).
//!
//! Los GIF reales (`image/gif`) se generan en memoria (figuras que se mueven sobre el degradado de
//! la paleta del mock) para ejercitar la animación de la UI; los «MP4» (como casi todos los de
//! Telegram) devuelven bytes que no son vídeo, así que la UI muestra su miniatura estática.
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder};
use tc_core::{FileKind, FileRef, GIF_MIME, Gif, GifPage, InlineResult, MediaVariant};

/// Ids de documento de los GIF del mock (no chocan con medios ni fotos de perfil).
pub const GIF_BASE: i64 = 880_000;
/// Resultados por página de la búsqueda.
pub const PAGE: usize = 6;
/// `query_id` de las búsquedas simuladas.
pub const QUERY_ID: i64 = 4242;

/// `(título, es GIF real, ancho, alto)`.
const SAMPLES: [(&str, bool, u32, u32); 10] = [
    ("gato bailando", true, 160, 120),
    ("aplausos", false, 240, 180),
    ("fuego hoguera", true, 120, 120),
    ("olas del mar", false, 320, 180),
    ("corazón latiendo", true, 120, 120),
    ("risa carcajada", false, 200, 200),
    ("lluvia", true, 160, 100),
    ("baile fiesta", false, 180, 240),
    ("hola saludo", true, 140, 140),
    ("pulgar arriba ok", false, 200, 150),
];

/// Cuántos de los de muestra están «guardados» en la cuenta (el resto solo sale al buscar).
pub const SAVED: usize = 6;
/// Fotogramas de cada GIF generado.
const FRAMES: u32 = 12;

fn is_gif_id(id: i64) -> bool {
    (GIF_BASE..GIF_BASE + SAMPLES.len() as i64).contains(&id)
}

fn sample(i: usize) -> Gif {
    let (title, real, w, h) = SAMPLES[i];
    let id = GIF_BASE + i as i64;
    let size = if real { u64::from(w * h) } else { 40_000 + i as u64 * 1000 };
    Gif {
        id,
        file: Some(FileRef {
            kind: FileKind::Document,
            id,
            access_hash: id.wrapping_mul(0x9e37_79b9),
            file_reference: id.to_le_bytes().to_vec(),
            dc_id: 2,
            size_bytes: size,
            thumb: Some("m".into()),
            full: None,
        }),
        mime: if real { GIF_MIME.into() } else { "video/mp4".into() },
        width: w,
        height: h,
        duration_secs: Some(2),
        title: title.into(),
        inline: None,
    }
}

/// GIFs guardados de la cuenta.
pub fn saved() -> Vec<Gif> {
    (0..SAVED).map(sample).collect()
}

/// Busca por palabras del título (sin distinguir mayúsculas). Vacía = todos. `offset` = índice.
pub fn search(query: &str, offset: &str) -> GifPage {
    let q = query.trim().to_lowercase();
    let all: Vec<Gif> = (0..SAMPLES.len())
        .map(sample)
        .filter(|g| q.is_empty() || q.split_whitespace().all(|w| g.title.to_lowercase().contains(w)))
        .map(|mut g| {
            g.inline = Some(InlineResult { query_id: QUERY_ID, result_id: format!("r{}", g.id) });
            g
        })
        .collect();
    let start = offset.parse::<usize>().unwrap_or(0).min(all.len());
    let end = (start + PAGE).min(all.len());
    GifPage { gifs: all[start..end].to_vec(), next_offset: (end < all.len()).then(|| end.to_string()) }
}

/// Un GIF de muestra por id (para enviarlo).
pub fn by_id(id: i64) -> Option<Gif> {
    is_gif_id(id).then(|| sample((id - GIF_BASE) as usize))
}

/// Fotograma `k` del GIF `i`: degradado con un disco que da una vuelta por ciclo.
fn frame(i: usize, k: u32, w: u32, h: u32) -> RgbaImage {
    const P: [[u8; 3]; 5] = [[214, 104, 64], [94, 140, 120], [72, 110, 160], [196, 160, 90], [150, 96, 150]];
    let (a, b) = (P[i % P.len()], P[(i + 2) % P.len()]);
    let t = k as f32 / FRAMES as f32 * std::f32::consts::TAU;
    let (cx, cy) = (w as f32 * (0.5 + 0.28 * t.cos()), h as f32 * (0.5 + 0.28 * t.sin()));
    let r = w.min(h) as f32 * 0.18;
    RgbaImage::from_fn(w, h, |x, y| {
        let u = (x + y) as f32 / (w + h) as f32;
        let mut c = [0u8; 3];
        for ((o, x), y) in c.iter_mut().zip(a).zip(b) {
            *o = (f32::from(x) * (1.0 - u) + f32::from(y) * u) as u8;
        }
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
        if d < r { Rgba([250, 244, 232, 255]) } else { Rgba([c[0], c[1], c[2], 255]) }
    })
}

/// GIF animado del de muestra `i`.
fn encode_gif(i: usize) -> Vec<u8> {
    let (_, _, w, h) = SAMPLES[i];
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new_with_speed(&mut out, 10);
        let _ = enc.set_repeat(Repeat::Infinite);
        let frames = (0..FRAMES).map(|k| Frame::from_parts(frame(i, k, w, h), 0, 0, Delay::from_numer_denom_ms(80, 1)));
        enc.encode_frames(frames).expect("GIF en memoria");
    }
    out
}

fn encode_png(img: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out).write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8).expect("PNG en memoria");
    out
}

/// Bytes de un GIF del mock (`None`: no es uno de ellos). La miniatura es el primer fotograma en
/// PNG; el archivo, el GIF animado o, en los «MP4», bytes que no son vídeo.
pub fn download(file: &FileRef, variant: MediaVariant) -> Option<Vec<u8>> {
    if !is_gif_id(file.id) {
        return None;
    }
    let i = (file.id - GIF_BASE) as usize;
    let (_, real, w, h) = SAMPLES[i];
    Some(match variant {
        MediaVariant::Thumb => encode_png(&frame(i, 0, w, h)),
        MediaVariant::Full if real => encode_gif(i),
        MediaVariant::Full => crate::images::document(file.id, file.size_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardados_busqueda_y_bytes() {
        let s = saved();
        assert_eq!(s.len(), SAVED);
        assert!(s.iter().any(Gif::is_real_gif) && s.iter().any(|g| !g.is_real_gif()));
        assert!(s.iter().all(|g| g.inline.is_none()));
        let p = search("", "");
        assert_eq!((p.gifs.len(), p.next_offset.as_deref()), (PAGE, Some("6")));
        let p2 = search("", "6");
        assert_eq!((p2.gifs.len(), p2.next_offset), (SAMPLES.len() - PAGE, None));
        let gato = search("GATO", "");
        assert_eq!(gato.gifs.len(), 1);
        assert_eq!(gato.gifs[0].inline.as_ref().map(|r| r.query_id), Some(QUERY_ID));
        let f = gato.gifs[0].file.clone().unwrap();
        let gif = download(&f, MediaVariant::Full).unwrap();
        assert_eq!(&gif[..4], b"GIF8");
        let dec = image::codecs::gif::GifDecoder::new(std::io::Cursor::new(gif)).unwrap();
        assert_eq!(image::AnimationDecoder::into_frames(dec).count(), FRAMES as usize);
        assert_eq!(&download(&f, MediaVariant::Thumb).unwrap()[..4], b"\x89PNG");
        let mp4 = s.iter().find(|g| !g.is_real_gif()).unwrap().file.clone().unwrap();
        assert_ne!(&download(&mp4, MediaVariant::Full).unwrap()[..4], b"GIF8");
        assert!(download(&FileRef { id: 5, ..f }, MediaVariant::Full).is_none());
        assert_eq!(by_id(GIF_BASE + 1).map(|g| g.title), Some("aplausos".into()));
    }

    #[tokio::test]
    async fn el_backend_envia_gifs_descargables_y_guarda_el_uso_de_emojis() {
        use tc_core::*;
        let b = crate::MockBackend::new(crate::MockConfig { now: Some(1_762_430_400), ..Default::default() });
        let g = b.saved_gifs().await.unwrap().remove(0);
        let m = b.send_gif(crate::BIG_CHAT, g.clone(), None, RandomId(77)).await.unwrap();
        let md = m.media.clone().unwrap();
        assert_eq!((md.kind, md.file_name.as_deref()), (MediaKind::Gif, Some("gato_bailando.gif")));
        let bytes = b.download_media(crate::BIG_CHAT, m.id, md.file.clone().unwrap(), MediaVariant::Full).await.unwrap();
        assert_eq!(&bytes[..4], b"GIF8");
        // Desde el panel (sin mensaje) también.
        assert_eq!(b.download_media(crate::BIG_CHAT, MsgId(0), md.file.unwrap(), MediaVariant::Thumb).await.unwrap()[..4], *b"\x89PNG");
        let unknown = Gif { id: 3, ..g };
        assert_eq!(b.send_gif(crate::BIG_CHAT, unknown, None, RandomId(78)).await, Err(BackendError::NotFound));

        b.record_emoji_use("😀".into(), Timestamp(10)).await.unwrap();
        b.record_emoji_use("👍".into(), Timestamp(20)).await.unwrap();
        b.record_emoji_use("😀".into(), Timestamp(5)).await.unwrap();
        let u = b.emoji_usage().await.unwrap();
        assert_eq!(u.iter().map(|e| (e.emoji.as_str(), e.count, e.last_used.0)).collect::<Vec<_>>(), [("👍", 1, 20), ("😀", 2, 10)]);
    }
}
