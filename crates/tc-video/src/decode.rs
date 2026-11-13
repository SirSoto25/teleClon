//! H.264 → fotogramas RGBA con OpenH264, siguiendo un [`Plan`] (fotogramas y tamaño).

use image::RgbaImage;
use image::imageops::FilterType;
use openh264::OpenH264API;
use openh264::decoder::{DecodedYUV, Decoder, DecoderConfig, Flush};
use openh264::formats::YUVSource;

use crate::VideoError;
use crate::mp4::{self, Track};
use crate::plan::{self, Limits, Plan};

/// Muestras como mucho que se miran de un clip (30 s a 30 fps); el resto se ignora.
pub const MAX_SAMPLES: usize = 900;

/// Un fotograma decodificado y escalado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// RGBA8, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
    /// Cuánto se muestra (ms).
    pub ms: u32,
}

/// Animación lista: fotogramas en orden de presentación.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub frames: Vec<Frame>,
}

impl Clip {
    pub fn total_ms(&self) -> u32 {
        self.frames.iter().fold(0u32, |a, f| a.saturating_add(f.ms))
    }
}

/// Decodifica la primera pista H.264 de un MP4 dentro de `limits`.
pub fn decode_mp4(bytes: &[u8], limits: &Limits) -> Result<Clip, VideoError> {
    let mut track = mp4::parse(bytes)?;
    track.samples.truncate(MAX_SAMPLES);
    let durations = mp4::display_durations_ms(&track);
    let plan = plan::plan(track.width, track.height, &durations, limits);
    let picks = plan::pick(durations.len(), plan.frames);
    let ms = plan::group_ms(&durations, &picks);
    decode_track(bytes, &track, plan, &picks, &ms)
}

struct Collector<'a> {
    plan: Plan,
    picks: &'a [usize],
    ms: &'a [u32],
    /// Fotogramas que ha devuelto el decodificador (orden de presentación).
    seen: usize,
    out: Vec<Frame>,
}

impl Collector<'_> {
    fn done(&self) -> bool {
        self.out.len() >= self.picks.len()
    }

    fn push(&mut self, yuv: &DecodedYUV<'_>) {
        let k = self.seen;
        self.seen += 1;
        let j = self.out.len();
        if self.picks.get(j) != Some(&k) {
            return;
        }
        let (w, h) = yuv.dimensions();
        let (w32, h32) = (w as u32, h as u32);
        if w == 0 || h == 0 {
            return;
        }
        let mut rgba = vec![0u8; w * h * 4];
        yuv.write_rgba8(&mut rgba);
        let (tw, th) = plan::fit(w32, h32, self.plan.side());
        let rgba = if (tw, th) == (w32, h32) {
            rgba
        } else {
            let Some(img) = RgbaImage::from_raw(w32, h32, rgba) else { return };
            image::imageops::resize(&img, tw, th, FilterType::Triangle).into_raw()
        };
        self.out.push(Frame { width: tw, height: th, rgba, ms: self.ms.get(j).copied().unwrap_or(40) });
    }
}

fn decode_track(file: &[u8], track: &Track, plan: Plan, picks: &[usize], ms: &[u32]) -> Result<Clip, VideoError> {
    let config = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
    let mut dec = Decoder::with_api_config(OpenH264API::from_source(), config).map_err(|e| VideoError::Decode(e.to_string()))?;
    let mut c = Collector { plan, picks, ms, seen: 0, out: Vec::new() };

    // Los SPS/PPS van en `avcC`, no en las muestras: se dan delante de la primera.
    let mut packet = Vec::new();
    for ps in track.sps.iter().chain(&track.pps) {
        packet.extend_from_slice(&[0, 0, 0, 1]);
        packet.extend_from_slice(ps);
    }
    let mut last_error = None;
    for s in &track.samples {
        if c.done() {
            break;
        }
        let data = track.data(file, s)?;
        mp4::annex_b(data, track.length_size, &mut packet)?;
        match dec.decode(&packet) {
            Ok(Some(yuv)) => c.push(&yuv),
            Ok(None) => {}
            // Un fotograma dañado no tira el clip; si no sale ninguno, se informa abajo.
            Err(e) => last_error = Some(e.to_string()),
        }
        packet.clear();
    }
    if !c.done() {
        match dec.flush_remaining() {
            Ok(rest) => {
                for yuv in &rest {
                    c.push(yuv);
                }
            }
            Err(e) => last_error = Some(e.to_string()),
        }
    }
    if c.out.is_empty() {
        return Err(VideoError::Decode(last_error.unwrap_or_else(|| "sin fotogramas".into())));
    }
    Ok(Clip { frames: c.out })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testmux::{encode_mp4, solid};

    const L: Limits = Limits { max_side: 320, min_side: 160, max_frames: 150, max_bytes: 10 << 20, min_frame_ms: 33 };

    fn center(f: &Frame) -> [u8; 4] {
        let i = ((f.height / 2 * f.width + f.width / 2) * 4) as usize;
        [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
    }

    fn dominant(px: [u8; 4]) -> usize {
        (0..3).max_by_key(|&i| px[i]).unwrap()
    }

    #[test]
    fn decodifica_un_mp4_codificado_con_openh264() {
        let colors = [[220, 30, 30], [30, 200, 30], [30, 30, 220], [220, 30, 30]];
        let frames: Vec<Vec<u8>> = colors.iter().map(|c| solid(64, 48, *c)).collect();
        let mp4 = encode_mp4(64, 48, &frames, 100);
        let clip = decode_mp4(&mp4, &L).expect("decodifica");
        assert_eq!(clip.frames.len(), 4);
        assert_eq!(clip.total_ms(), 400);
        for (f, want) in clip.frames.iter().zip([0, 1, 2, 0]) {
            assert_eq!((f.width, f.height, f.rgba.len()), (64, 48, 64 * 48 * 4));
            assert_eq!(dominant(center(f)), want, "color del fotograma: {:?}", center(f));
            assert_eq!(center(f)[3], 255);
        }
    }

    #[test]
    fn escala_y_reparte_segun_el_plan() {
        let frames: Vec<Vec<u8>> = (0..12u8).map(|k| solid(96, 64, [k * 20, 100, 200 - k * 15])).collect();
        let mp4 = encode_mp4(96, 64, &frames, 40);
        // Lado máximo 48 y sitio para 5 fotogramas de 48×32.
        let tight = Limits { max_side: 48, min_side: 48, max_frames: 150, max_bytes: 5 * 48 * 32 * 4, min_frame_ms: 0 };
        let clip = decode_mp4(&mp4, &tight).unwrap();
        assert_eq!(clip.frames.len(), 5);
        assert!(clip.frames.iter().all(|f| (f.width, f.height) == (48, 32)));
        assert_eq!(clip.total_ms(), 12 * 40, "el bucle dura lo mismo");
    }

    #[test]
    fn basura_o_vacio_es_error() {
        assert!(decode_mp4(b"", &L).is_err());
        assert!(decode_mp4(b"\0\0\0\x08ftyp", &L).is_err());
        let mp4 = encode_mp4(16, 16, &[solid(16, 16, [1, 2, 3])], 40);
        // Muestras corrompidas: error, no pánico.
        let mut bad = mp4.clone();
        let mdat = bad.windows(4).position(|w| w == b"mdat").unwrap() + 4;
        for b in &mut bad[mdat + 8..mdat + 40] {
            *b = 0xff;
        }
        let _ = decode_mp4(&bad, &L);
    }
}
