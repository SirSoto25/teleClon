//! Pruebas de la animación de TGS (ADR-0008): índice de fotograma, política de movimiento, LRU por
//! bytes y, con la feature `lottie`, render real de un Lottie mínimo generado aquí mismo.

use std::io::Write;
use std::time::Duration;

use crate::anim::{self, Playback, Show, Tgs};
use crate::tgs::TgsInfo;
use crate::{ByteLru, StickerError};

/// Lottie mínimo: cuadrado rojo de 20 px que se desplaza de x=16 a x=48 en 60 fotogramas a 60 fps.
const MOVING: &str = r#"{"v":"5.5.2","fr":60,"ip":0,"op":60,"w":64,"h":64,"nm":"m","ddd":0,"assets":[],"layers":[{"ddd":0,"ind":1,"ty":4,"nm":"s","sr":1,"ks":{"o":{"a":0,"k":100},"r":{"a":0,"k":0},"p":{"a":1,"k":[{"t":0,"s":[16,32,0],"e":[48,32,0],"i":{"x":[1],"y":[1]},"o":{"x":[0],"y":[0]}},{"t":59}]},"a":{"a":0,"k":[0,0,0]},"s":{"a":0,"k":[100,100,100]}},"ao":0,"shapes":[{"ty":"gr","it":[{"ty":"rc","d":1,"s":{"a":0,"k":[20,20]},"p":{"a":0,"k":[0,0]},"r":{"a":0,"k":0}},{"ty":"fl","c":{"a":0,"k":[1,0,0,1]},"o":{"a":0,"k":100},"r":1},{"ty":"tr","p":{"a":0,"k":[0,0]},"a":{"a":0,"k":[0,0]},"s":{"a":0,"k":[100,100]},"r":{"a":0,"k":0},"o":{"a":0,"k":100}}]}],"ip":0,"op":60,"st":0,"bm":0}]}"#;

/// TGS = gzip del JSON, comprimido aquí con flate2 (sin pasar por `tgs::compress`).
fn gzip(json: &str) -> Vec<u8> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    enc.write_all(json.as_bytes()).unwrap();
    enc.finish().unwrap()
}

fn info(frames: usize, fps: f64) -> TgsInfo {
    TgsInfo { frames, fps, width: 512, height: 512 }
}

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

#[test]
fn fotograma_por_tiempo_a_30_fps_como_mucho() {
    // 60 fps de origen: se muestra uno de cada dos (30 fps), en bucle de 3 s.
    let i = info(180, 60.0);
    assert_eq!(anim::frame_step(&i), 2);
    assert_eq!(anim::frame_at(&i, ms(0)), 0);
    assert_eq!(anim::frame_at(&i, ms(34)), 2);
    assert_eq!(anim::frame_at(&i, ms(1500)), 90);
    assert_eq!(anim::frame_at(&i, ms(3000)), 0, "vuelve al principio");
    assert_eq!(anim::frame_at(&i, ms(3034)), 2);
    // 30 fps o menos: todos los fotogramas.
    let j = info(30, 30.0);
    assert_eq!(anim::frame_step(&j), 1);
    assert_eq!(anim::frame_at(&j, ms(500)), 15);
    assert_eq!(anim::frame_at(&info(48, 24.0), ms(1000)), 24);
    // Duración que no es múltiplo del paso: siempre los mismos fotogramas (pares) en cada vuelta.
    let k = info(61, 60.0);
    let mut seen = std::collections::BTreeSet::new();
    for t in (0..20_000).step_by(7) {
        let f = anim::frame_at(&k, ms(t));
        assert!(f < 61 && f.is_multiple_of(2), "fotograma {f}");
        seen.insert(f);
    }
    assert_eq!(seen.len(), 31);
    // Metadatos rotos: siempre el primero.
    assert_eq!(anim::frame_at(&info(0, 60.0), ms(900)), 0);
    assert_eq!(anim::frame_at(&info(10, f64::NAN), ms(900)), 0);
}

#[test]
fn siguientes_fotogramas_en_bucle() {
    let i = info(10, 60.0);
    assert_eq!(anim::next_frames(&i, 6, 3), vec![8, 0, 2]);
    assert_eq!(anim::next_frames(&info(2, 30.0), 0, 5), vec![1], "sin repetir el actual");
    assert!(anim::next_frames(&info(0, 30.0), 0, 5).is_empty());
}

#[test]
fn movimiento_reducido_foco_y_visibilidad() {
    let i = info(180, 60.0);
    let full = Playback { reduced_motion: false, focused: true };
    assert_eq!(full.show(&i, true, ms(1500)), Show::Frame(90));
    assert_eq!(full.show(&i, false, ms(1500)), Show::Still, "fuera de la vista no se anima");
    assert!(full.ticks(true));
    assert!(!full.ticks(false), "sin nada visible no hay reloj");

    let reduced = Playback { reduced_motion: true, focused: true };
    assert_eq!(reduced.show(&i, true, ms(1500)), Show::Still, "movimiento reducido: primer fotograma");
    assert!(!reduced.ticks(true));

    let unfocused = Playback { reduced_motion: false, focused: false };
    assert_eq!(unfocused.show(&i, true, ms(1500)), Show::Paused, "sin foco se congela");
    assert!(!unfocused.ticks(true));
}

#[test]
fn lru_por_bytes() {
    let mut c: ByteLru<u32, &str> = ByteLru::new(100);
    assert!(c.insert(1, "a", 40));
    assert!(c.insert(2, "b", 40));
    assert_eq!(c.get(&1), Some(&"a"), "1 pasa a ser el más reciente");
    assert!(c.insert(3, "c", 40));
    assert!(!c.contains(&2), "se expulsa el menos usado");
    assert_eq!(c.used_bytes(), 80);
    assert!(!c.insert(4, "enorme", 101), "mayor que el tope: no se guarda");
    assert_eq!(c.len(), 2);
    assert!(c.insert(1, "a2", 10), "reemplazar descuenta el peso anterior");
    assert_eq!((c.used_bytes(), c.peek(&1)), (50, Some(&"a2")));
    c.retain(|k| *k != 3);
    assert_eq!((c.len(), c.used_bytes()), (1, 10));
    c.clear();
    assert!(c.is_empty() && c.used_bytes() == 0);
}

#[test]
fn tgs_invalido_se_rechaza() {
    assert!(matches!(Tgs::load(b"no es gzip", "k"), Err(StickerError::Gzip(_))));
    assert!(matches!(Tgs::load(&gzip("hola"), "k"), Err(StickerError::NotLottie)));
    assert!(matches!(Tgs::load(&gzip(MOVING), ""), Err(StickerError::NotLottie)), "clave vacía");
}

#[cfg(not(feature = "lottie"))]
#[test]
fn sin_lottie_no_hay_animacion() {
    assert!(matches!(Tgs::load(&gzip(MOVING), "anim-off"), Err(StickerError::LottieDisabled)));
}

#[cfg(feature = "lottie")]
#[test]
fn tgs_generado_se_decodifica_y_se_mueve() {
    let t = Tgs::load(&gzip(MOVING), "anim-test-moving").expect("Lottie válido");
    assert_eq!(t.info.frames, 60);
    assert!((t.info.fps - 60.0).abs() < 1e-9);
    assert_eq!((t.info.width, t.info.height), (64, 64));
    assert!(t.byte_len() >= MOVING.len());

    let px = 32;
    let out = t.render_many(&[0, 59, 60], px);
    assert_eq!(out.len(), 3);
    let first = out[0].1.as_ref().expect("fotograma 0");
    let last = out[1].1.as_ref().expect("fotograma 59");
    assert!(matches!(out[2].1, Err(StickerError::FrameOutOfRange(60))));
    assert_eq!((first.width, first.height, first.rgba.len()), (px, px, (px * px * 4) as usize));
    // Al principio el cuadrado está a la izquierda (x≈16 de 64); al final, a la derecha (x≈48).
    let left = (8, 16);
    let right = (24, 16);
    assert_eq!(first.pixel(left.0, left.1).unwrap(), [255, 0, 0, 255]);
    assert_eq!(first.pixel(right.0, right.1).unwrap()[3], 0);
    assert_eq!(last.pixel(right.0, right.1).unwrap(), [255, 0, 0, 255]);
    assert_eq!(last.pixel(left.0, left.1).unwrap()[3], 0);
    // Un fotograma suelto y tamaños fuera de rango.
    assert_eq!(&t.render(0, px).unwrap(), first);
    assert!(matches!(t.render(0, 0), Err(StickerError::BadSize)));
    assert!(matches!(t.render(0, 4096), Err(StickerError::BadSize)));
}
