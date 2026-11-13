//! MP4 real hecho con x264 (perfil High, CABAC, fotogramas B con `ctts`, `moov` al principio), como
//! los que convierte Telegram. El brillo sube en cada fotograma, así se comprueba el orden de
//! presentación. Se generó una vez con:
//!
//! ```text
//! ffmpeg -f lavfi -i "color=c=black:s=64x48:r=10:d=1,format=yuv420p,geq=lum='20+N*22+mod(X+Y+N*3\,16)':cb=128:cr=128" \
//!   -c:v libx264 -profile:v high -x264-params bframes=2:b-adapt=0:scenecut=0:keyint=10 -crf 30 \
//!   -pix_fmt yuv420p -movflags +faststart -an -map_metadata -1 -fflags +bitexact -flags:v +bitexact \
//!   high_bframes_64x48.mp4
//! ```
//!
//! Tipos de fotograma (orden de presentación): I B B P B B P B B P.
use tc_video::{Limits, decode_mp4, looks_like_mp4, mp4};

const HIGH_BFRAMES: &[u8] = include_bytes!("fixtures/high_bframes_64x48.mp4");
const L: Limits = Limits { max_side: 320, min_side: 160, max_frames: 150, max_bytes: 10 << 20, min_frame_ms: 33 };

#[test]
fn perfil_high_con_fotogramas_b() {
    assert!(looks_like_mp4(HIGH_BFRAMES));
    let t = mp4::parse(HIGH_BFRAMES).unwrap();
    assert_eq!((t.width, t.height, t.samples.len()), (64, 48, 10));
    assert!(t.samples.iter().any(|s| s.cts != t.samples[0].cts), "tiene ctts (fotogramas B)");
    assert_eq!(mp4::display_durations_ms(&t), [100; 10]);

    let clip = decode_mp4(HIGH_BFRAMES, &L).expect("OpenH264 decodifica High con B");
    assert_eq!(clip.frames.len(), 10);
    assert_eq!(clip.total_ms(), 1000);
    assert!(clip.frames.iter().all(|f| (f.width, f.height, f.rgba.len()) == (64, 48, 64 * 48 * 4)));
    // El brillo medio sube fotograma a fotograma: el decodificador los da en orden de presentación.
    let luma: Vec<u64> = clip.frames.iter().map(|f| f.rgba.chunks(4).map(|p| u64::from(p[1])).sum::<u64>() / (64 * 48)).collect();
    assert!(luma.windows(2).all(|w| w[1] > w[0] + 10), "orden de presentación: {luma:?}");
}
