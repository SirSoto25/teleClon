//! Imágenes sintéticas del mock: PNG válidos generados en memoria (degradado diagonal con bandas
//! de curvas de nivel), deterministas por id. Sirven para ejercitar la descarga, la caché y el
//! decodificado de la UI sin red ni archivos.
use image::{ImageEncoder, codecs::png::PngEncoder};
use tc_core::{FileKind, FileRef, MediaKind, MediaVariant, PhotoSize, ProfilePhoto};

/// Desplazamiento de los ids de foto de perfil del mock (no chocan con los de medios).
const PHOTO_BASE: i64 = 600_000;

/// Foto de perfil determinista: dos de cada tres usuarios/grupos tienen una.
pub fn profile_photo(owner: i64) -> Option<ProfilePhoto> {
    (owner.rem_euclid(3) != 0).then_some(ProfilePhoto { id: PHOTO_BASE + owner, dc_id: 2 })
}

/// Referencia descargable para un medio del mock.
pub fn file_ref(kind: MediaKind, id: i64, size_bytes: u64) -> FileRef {
    let photo = kind == MediaKind::Photo;
    FileRef {
        kind: if photo { FileKind::Photo } else { FileKind::Document },
        id,
        access_hash: id.wrapping_mul(0x9e37_79b9),
        file_reference: id.to_le_bytes().to_vec(),
        dc_id: 2,
        size_bytes,
        thumb: photo.then(|| "x".to_string()),
        full: photo.then(|| "y".to_string()),
    }
}

/// Lado en px de una foto de perfil sintética.
pub fn profile_side(size: PhotoSize) -> u32 {
    match size {
        PhotoSize::Small => 160,
        PhotoSize::Big => 640,
    }
}

/// Dimensiones de la variante pedida de una foto `w`×`h`: la vista previa cabe en 800 px y la
/// completa en 1600 px (sin ampliar nunca).
pub fn variant_dims(w: u32, h: u32, v: MediaVariant) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    let cap = match v {
        MediaVariant::Thumb => 800,
        MediaVariant::Full => 1600,
    };
    let k = (cap as f32 / w.max(h) as f32).min(1.0);
    (((w as f32 * k).round() as u32).max(1), ((h as f32 * k).round() as u32).max(1))
}

/// Color de la paleta de muestra (tierra/ascua) para una semilla.
fn tone(seed: u64) -> [f32; 3] {
    const P: [[u8; 3]; 8] = [
        [214, 104, 64],
        [186, 132, 72],
        [94, 140, 120],
        [72, 110, 160],
        [150, 96, 150],
        [196, 160, 90],
        [120, 150, 80],
        [170, 90, 90],
    ];
    let c = P[(seed % P.len() as u64) as usize];
    [f32::from(c[0]), f32::from(c[1]), f32::from(c[2])]
}

/// PNG de `w`×`h` px para `seed`.
pub fn png(seed: i64, w: u32, h: u32) -> Vec<u8> {
    let (w, h) = (w.max(1), h.max(1));
    let s = seed.unsigned_abs();
    let (a, b) = (tone(s), tone(s / 7 + 3));
    let freq = 6.0 + (s % 5) as f32;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (u, v) = (x as f32 / w as f32, y as f32 / h as f32);
            let t = (u + v) / 2.0;
            // Bandas suaves (curvas de nivel de una función radial) sobre el degradado.
            let r = ((u - 0.3).powi(2) + (v - 0.35).powi(2)).sqrt();
            let band = if (r * freq).fract() < 0.08 { 0.82 } else { 1.0 };
            for i in 0..3 {
                rgba.push(((a[i] * (1.0 - t) + b[i] * t) * band).clamp(0.0, 255.0) as u8);
            }
            rgba.push(255);
        }
    }
    let mut out = Vec::new();
    PngEncoder::new(&mut out).write_image(&rgba, w, h, image::ExtendedColorType::Rgba8).expect("PNG en memoria");
    out
}

/// Contenido de un documento del mock (no es una imagen).
pub fn document(id: i64, size_bytes: u64) -> Vec<u8> {
    let n = size_bytes.min(64 * 1024) as usize;
    (0..n).map(|i| (i as i64).wrapping_mul(31).wrapping_add(id) as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_is_valid_and_deterministic() {
        let a = png(42, 32, 20);
        assert_eq!(&a[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(a, png(42, 32, 20));
        let img = image::load_from_memory(&a).unwrap();
        assert_eq!((img.width(), img.height()), (32, 20));
    }

    #[test]
    fn variants_never_upscale() {
        assert_eq!(variant_dims(1280, 720, MediaVariant::Thumb), (800, 450));
        assert_eq!(variant_dims(1280, 720, MediaVariant::Full), (1280, 720));
        assert_eq!(variant_dims(3200, 1600, MediaVariant::Full), (1600, 800));
        assert_eq!(profile_photo(3), None);
        assert_eq!(profile_photo(4).map(|p| p.id), Some(PHOTO_BASE + 4));
    }
}
