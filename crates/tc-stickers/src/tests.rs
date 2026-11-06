use tc_core::StickerId;

use crate::{CacheKey, Favorites, Frame, FrameCache, Recents, StickerError, pack, tgs, webp};

fn gradient(w: u32, h: u32) -> Frame {
    let mut rgba = Vec::new();
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&[(x * 255 / (w - 1)) as u8, (y * 255 / (h - 1)) as u8, 77, if x == 0 { 128 } else { 255 }]);
        }
    }
    Frame { width: w, height: h, rgba }
}

#[test]
fn webp_roundtrip_is_lossless() {
    let f = gradient(37, 21);
    let bytes = webp::encode_lossless(&f).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    let back = webp::decode(&bytes).unwrap();
    assert_eq!(back, f);
    assert_eq!(back.pixel(36, 20), Some([255, 255, 77, 255]));
    assert_eq!(back.pixel(37, 0), None);
}

#[test]
fn webp_garbage_is_error() {
    assert!(matches!(webp::decode(b"no soy un webp"), Err(StickerError::Image(_))));
    let f = gradient(8, 8);
    let bytes = webp::encode_lossless(&f).unwrap();
    assert!(webp::decode(&bytes[..bytes.len() / 2]).is_err());
}

const LOTTIE: &str = r#"{"v":"5.5.2","fr":30,"ip":0,"op":30,"w":64,"h":64,"nm":"t","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},"p":{"a":0,"k":[32,32,0]},"a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},"ao":0,"shapes":[{"ty":"gr","it":[{"ty":"rc","d":1,"s":{"a":0,"k":[40,40]},"p":{"a":0,"k":[0,0]},"r":{"a":0,"k":0}},{"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},{"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}]}],"ip":0,"op":30,"st":0,"bm":0}]}"#;

#[test]
fn tgs_roundtrip_and_validation() {
    let tgs_bytes = tgs::compress(LOTTIE.as_bytes());
    assert_eq!(tgs::decompress(&tgs_bytes).unwrap(), LOTTIE.as_bytes());
    assert!(matches!(tgs::decompress(b"plain"), Err(StickerError::Gzip(_))));
    assert!(matches!(tgs::decompress(&tgs::compress(b"hola")), Err(StickerError::NotLottie)));
}

#[test]
fn tgs_bomb_is_rejected() {
    let bomb = tgs::compress(&vec![b'{'; tgs::MAX_UNCOMPRESSED + 10]);
    assert!(bomb.len() < 100_000);
    assert!(matches!(tgs::decompress(&bomb), Err(StickerError::TooLarge(_))));
}

#[cfg(not(feature = "lottie"))]
#[test]
fn render_without_lottie_reports_disabled() {
    assert!(!tgs::lottie_enabled());
    let t = tgs::compress(LOTTIE.as_bytes());
    assert!(matches!(tgs::render(&t, "k", 0, 32, 32), Err(StickerError::LottieDisabled)));
}

#[cfg(feature = "lottie")]
#[test]
fn render_with_lottie() {
    let t = tgs::compress(LOTTIE.as_bytes());
    let info = tgs::info(&t, "k-info").unwrap();
    assert_eq!(info.frames, 30);
    let f = tgs::render(&t, "k-render", 0, 64, 64).unwrap();
    assert_eq!(f.pixel(32, 32).unwrap(), [255, 0, 0, 255]);
    assert_eq!(f.pixel(2, 2).unwrap()[3], 0);
    assert!(matches!(tgs::render(&t, "k-render", 99, 64, 64), Err(StickerError::FrameOutOfRange(99))));
}

fn key(id: i64) -> CacheKey {
    CacheKey { sticker_id: id, size: 2, frame: 0 }
}

#[test]
fn cache_evicts_least_recently_used_by_bytes() {
    let f = || Frame::transparent(2, 2); // 16 bytes
    let mut c = FrameCache::new(48);
    c.insert(key(1), f());
    c.insert(key(2), f());
    c.insert(key(3), f());
    assert_eq!((c.len(), c.used_bytes()), (3, 48));
    assert!(c.get(&key(1)).is_some()); // 2 pasa a ser el más viejo
    c.insert(key(4), f());
    assert!(!c.contains(&key(2)));
    assert!(c.contains(&key(1)) && c.contains(&key(3)) && c.contains(&key(4)));
    assert_eq!(c.used_bytes(), 48);
    // Reemplazar no duplica bytes.
    c.insert(key(4), f());
    assert_eq!(c.used_bytes(), 48);
    // Mayor que el tope: no se guarda.
    let big = c.insert(key(9), Frame::transparent(10, 10));
    assert_eq!(big.width, 10);
    assert!(!c.contains(&key(9)));
    assert_eq!(c.used_bytes(), 48);
    c.clear();
    assert!(c.is_empty() && c.used_bytes() == 0);
}

#[test]
fn cache_get_or_insert_computes_once() {
    let mut c = FrameCache::default();
    let mut calls = 0;
    for _ in 0..3 {
        c.get_or_insert_with::<()>(key(1), || {
            calls += 1;
            Ok(Frame::transparent(1, 1))
        })
        .unwrap();
    }
    assert_eq!(calls, 1);
    assert_eq!(c.get_or_insert_with(key(2), || Err("x")), Err("x"));
}

#[test]
fn recents_and_favorites() {
    let id = |n| StickerId(n);
    let mut r = Recents::new(3);
    for n in [1, 2, 3, 2, 4] {
        r.record(id(n));
    }
    assert_eq!(r.ids(), &[id(4), id(2), id(3)]);
    r.remove(id(2));
    assert_eq!(r.ids(), &[id(4), id(3)]);
    assert_eq!(Recents::from_ids(2, [id(5), id(5), id(6), id(7)]).ids(), &[id(5), id(6)]);
    let mut f = Favorites::from_ids([id(1), id(1), id(2)]);
    assert_eq!(f.ids(), &[id(1), id(2)]);
    assert!(f.toggle(id(3)));
    assert!(!f.toggle(id(1)));
    assert!(f.contains(id(3)) && !f.contains(id(1)));
}

#[test]
fn pack_has_twelve_original_stickers() {
    let set = pack::sample_set();
    assert_eq!(set.stickers.len(), 12);
    assert_eq!(pack::count(), 12);
    let mut ids: Vec<_> = set.stickers.iter().map(|s| s.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 12);
    for (i, s) in set.stickers.iter().enumerate() {
        assert_eq!(pack::index_of(s.id), Some(i));
        let f = pack::render(i, 64).unwrap();
        assert_eq!((f.width, f.height, f.byte_len()), (64, 64, 64 * 64 * 4));
        let opaque = f.rgba.chunks(4).filter(|p| p[3] == 255).count();
        let clear = f.rgba.chunks(4).filter(|p| p[3] == 0).count();
        assert!(opaque > 64 * 64 / 20, "{} casi vacío", pack::sticker_name(i).unwrap());
        assert!(clear > 64 * 64 / 10, "{} sin fondo transparente", pack::sticker_name(i).unwrap());
        assert_eq!(f.pixel(0, 0).unwrap()[3], 0, "esquina transparente");
    }
    assert!(pack::render(12, 64).is_none() && pack::render(0, 0).is_none());
    assert_eq!(pack::index_of(StickerId(5)), None);
}

#[test]
fn pack_stickers_are_distinct_and_survive_webp() {
    let a = pack::render(0, 48).unwrap();
    let b = pack::render(1, 48).unwrap();
    assert_ne!(a, b);
    // El corazón usa el rojo de la paleta; el centro del check usa el verde de éxito.
    let heart = pack::render(1, 64).unwrap().pixel(32, 40).unwrap();
    assert_eq!(&heart[..3], &[tc_theme::palette::DARK.danger_solid.r, tc_theme::palette::DARK.danger_solid.g, tc_theme::palette::DARK.danger_solid.b]);
    let bytes = webp::encode_lossless(&a).unwrap();
    assert_eq!(webp::decode(&bytes).unwrap(), a);
}
