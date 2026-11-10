//! Decodificado en streaming a muestras `f32` intercaladas.
//!
//! [`open`] elige el decodificador por el contenido (no por la extensión): OGG con `OpusHead` va a
//! [`OpusStream`]; el resto, a symphonia (MP3, M4A/AAC, FLAC, WAV y OGG/Vorbis), con el tipo MIME como
//! pista. Los bytes se comparten (`Arc<[u8]>`): nada se copia entero salvo los paquetes Opus.
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use crate::AudioError;
use crate::ogg::{self, OPUS_RATE};

/// Fuente de audio que se decodifica a trozos. Se mueve al hilo de audio (`Send`).
pub trait AudioStream: Send {
    fn sample_rate(&self) -> u32;
    /// Canales (1 o 2 en la práctica). Constante durante todo el flujo.
    fn channels(&self) -> u16;
    /// Duración total si se conoce.
    fn duration(&self) -> Option<Duration>;
    /// Vacía `out` y la llena con el siguiente trozo (frames completos). `false` al terminar.
    fn next_chunk(&mut self, out: &mut Vec<f32>) -> Result<bool, AudioError>;
    /// Salta a `to` (se recorta a la duración). Lo siguiente que da `next_chunk` empieza ahí.
    fn seek(&mut self, to: Duration) -> Result<(), AudioError>;
}

/// Audio entero en memoria (pruebas y formas de onda calculadas).
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Pcm {
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.frames() as f64 / f64::from(self.sample_rate.max(1)))
    }
}

/// Abre un decodificador para `bytes`. `mime`: pista opcional (`audio/mpeg`, `audio/mp4`…).
pub fn open(bytes: Arc<[u8]>, mime: Option<&str>) -> Result<Box<dyn AudioStream>, AudioError> {
    if ogg::is_ogg_opus(&bytes) {
        return Ok(Box::new(OpusStream::new(&bytes)?));
    }
    Ok(Box::new(SymphoniaStream::new(bytes, mime)?))
}

/// Decodifica todo a memoria.
pub fn decode_all(bytes: Arc<[u8]>, mime: Option<&str>) -> Result<Pcm, AudioError> {
    let mut s = open(bytes, mime)?;
    let (sample_rate, channels) = (s.sample_rate(), s.channels());
    let (mut samples, mut chunk) = (Vec::new(), Vec::new());
    while s.next_chunk(&mut chunk)? {
        samples.extend_from_slice(&chunk);
    }
    Ok(Pcm { sample_rate, channels, samples })
}

// ------------------------------------------------------------------------------------------ Opus

/// Pre-roll recomendado tras un salto (RFC 7845 §4.6): 80 ms.
const PREROLL: u64 = 3840;
/// Mayor paquete Opus: 120 ms a 48 kHz.
const MAX_PACKET: usize = 5760;

/// OGG/Opus con `opus-decoder` (Rust puro). Posiciones en muestras a 48 kHz contando el `pre_skip`.
pub struct OpusStream {
    packets: Vec<Vec<u8>>,
    /// Posición de inicio de cada paquete (y una más al final).
    starts: Vec<u64>,
    dec: opus_decoder::OpusDecoder,
    channels: u16,
    gain: f32,
    /// Siguiente paquete y posición de su primera muestra.
    idx: usize,
    pos: u64,
    /// Se descartan las muestras anteriores a esto (pre_skip y pre-roll tras un salto).
    skip_until: u64,
    /// Fin del audio útil (última posición granular).
    end: u64,
    pre_skip: u64,
    scratch: Vec<f32>,
    errors: usize,
}

impl OpusStream {
    fn new(bytes: &[u8]) -> Result<Self, AudioError> {
        let parsed = ogg::parse(bytes)?;
        let h = parsed.head;
        if h.mapping_family != 0 || h.channels > 2 {
            // Multicanal (familia 1+): no aparece en notas de voz ni en música normal.
            return Err(AudioError::Unsupported);
        }
        let channels = u16::from(h.channels);
        let dec = opus_decoder::OpusDecoder::new(OPUS_RATE, usize::from(channels)).map_err(|e| AudioError::Corrupt(e.to_string()))?;
        let mut starts = Vec::with_capacity(parsed.packets.len() + 1);
        let mut acc = 0u64;
        starts.push(0);
        for p in &parsed.packets {
            acc += ogg::packet_samples(p);
            starts.push(acc);
        }
        let pre_skip = u64::from(h.pre_skip);
        let end = parsed.last_granule.filter(|g| *g > pre_skip && *g <= acc).unwrap_or(acc);
        Ok(Self {
            packets: parsed.packets,
            starts,
            dec,
            channels,
            gain: 10f32.powf(f32::from(h.gain_q8) / 256.0 / 20.0),
            idx: 0,
            pos: 0,
            skip_until: pre_skip,
            end,
            pre_skip,
            scratch: vec![0.0; MAX_PACKET * usize::from(channels)],
            errors: 0,
        })
    }
}

impl AudioStream for OpusStream {
    fn sample_rate(&self) -> u32 {
        OPUS_RATE
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(self.end.saturating_sub(self.pre_skip) as f64 / f64::from(OPUS_RATE)))
    }

    fn next_chunk(&mut self, out: &mut Vec<f32>) -> Result<bool, AudioError> {
        out.clear();
        let ch = usize::from(self.channels);
        while self.idx < self.packets.len() && self.pos < self.end {
            let packet = &self.packets[self.idx];
            self.idx += 1;
            let (dec, scratch) = (&mut self.dec, &mut self.scratch);
            // opus-decoder es joven: un pánico en un paquete raro no puede tumbar el hilo de audio.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dec.decode_float(packet, scratch, false)));
            let n = match r {
                Ok(Ok(n)) => n,
                failed => {
                    // Paquete dañado: silencio de su duración para no desfasar el tiempo.
                    self.errors += 1;
                    let why = match failed {
                        Ok(Err(e)) => e.to_string(),
                        _ => "pánico del decodificador".to_string(),
                    };
                    tracing::debug!(error = %why, "paquete Opus no válido");
                    if self.errors > 50 && self.errors * 2 > self.idx {
                        return Err(AudioError::Corrupt(why));
                    }
                    self.dec.reset();
                    let n = (ogg::packet_samples(packet) as usize).min(MAX_PACKET);
                    self.scratch[..n * ch].fill(0.0);
                    n
                }
            };
            let start = self.pos;
            self.pos += n as u64;
            let from = self.skip_until.saturating_sub(start).min(n as u64) as usize;
            let to = (self.end.saturating_sub(start)).min(n as u64) as usize;
            if to > from {
                out.extend(self.scratch[from * ch..to * ch].iter().map(|s| s * self.gain));
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn seek(&mut self, to: Duration) -> Result<(), AudioError> {
        let target = (self.pre_skip + (to.as_secs_f64() * f64::from(OPUS_RATE)) as u64).min(self.end);
        let from = target.saturating_sub(PREROLL);
        // Último paquete que empieza en o antes de `from`.
        let k = self.starts.partition_point(|&s| s <= from).saturating_sub(1).min(self.packets.len());
        self.dec.reset();
        self.idx = k;
        self.pos = self.starts[k];
        self.skip_until = target.max(self.pre_skip);
        Ok(())
    }
}

// ------------------------------------------------------------------------------------- symphonia

/// MP3, M4A/AAC, FLAC, WAV y OGG/Vorbis con symphonia.
pub struct SymphoniaStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track: u32,
    rate: u32,
    channels: u16,
    duration: Option<Duration>,
    /// Trozo ya decodificado al abrir (para conocer el formato de salida).
    pending: Option<Vec<f32>>,
    /// Frames a descartar tras un salto preciso.
    skip_frames: u64,
    buf: Option<SampleBuffer<f32>>,
}

fn sym_err(e: SymError) -> AudioError {
    match e {
        SymError::Unsupported(_) => AudioError::Unsupported,
        e => AudioError::Corrupt(e.to_string()),
    }
}

impl SymphoniaStream {
    fn new(bytes: Arc<[u8]>, mime: Option<&str>) -> Result<Self, AudioError> {
        let mss = MediaSourceStream::new(Box::new(Cursor::new(bytes)), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if let Some(m) = mime {
            hint.mime_type(m);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions { enable_gapless: true, ..Default::default() }, &MetadataOptions::default())
            .map_err(|e| match e {
                SymError::Unsupported(_) | SymError::IoError(_) => AudioError::Unsupported,
                e => sym_err(e),
            })?;
        let format = probed.format;
        let (track_id, params) = {
            let t = format.tracks().iter().find(|t| t.codec_params.codec != CODEC_TYPE_NULL).ok_or(AudioError::Unsupported)?;
            (t.id, t.codec_params.clone())
        };
        let decoder = symphonia::default::get_codecs().make(&params, &DecoderOptions::default()).map_err(sym_err)?;
        let duration = match (params.n_frames, params.time_base) {
            (Some(n), Some(tb)) => {
                let t = tb.calc_time(n);
                Some(Duration::from_secs_f64(t.seconds as f64 + t.frac))
            }
            (Some(n), None) => params.sample_rate.map(|r| Duration::from_secs_f64(n as f64 / f64::from(r))),
            _ => None,
        };
        let mut s = Self {
            format,
            decoder,
            track: track_id,
            rate: params.sample_rate.unwrap_or(0),
            channels: params.channels.map_or(0, |c| c.count() as u16),
            duration,
            pending: None,
            skip_frames: 0,
            buf: None,
        };
        // Primer trozo: confirma que se decodifica y fija el formato de salida real.
        let mut first = Vec::new();
        let more = s.decode_next(&mut first, true)?;
        if !more && first.is_empty() {
            return Err(AudioError::Corrupt("sin audio".into()));
        }
        if s.rate == 0 || s.channels == 0 {
            return Err(AudioError::Unsupported);
        }
        s.pending = Some(first);
        Ok(s)
    }

    /// Decodifica el siguiente paquete con audio. `learn`: fija `rate`/`channels` con el primero.
    fn decode_next(&mut self, out: &mut Vec<f32>, learn: bool) -> Result<bool, AudioError> {
        let mut bad = 0u32;
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(false),
                Err(SymError::ResetRequired) => return Ok(false),
                Err(e) => return Err(sym_err(e)),
            };
            if packet.track_id() != self.track {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymError::DecodeError(e)) => {
                    bad += 1;
                    tracing::debug!(error = e, "paquete de audio no válido");
                    if bad > 100 {
                        return Err(AudioError::Corrupt(e.into()));
                    }
                    continue;
                }
                Err(e) => return Err(sym_err(e)),
            };
            if decoded.frames() == 0 {
                continue;
            }
            let spec = *decoded.spec();
            if learn {
                self.rate = spec.rate;
                self.channels = spec.channels.count() as u16;
            }
            copy_out(&mut self.buf, self.channels, decoded, out);
            if self.skip_frames > 0 {
                let ch = usize::from(self.channels.max(1));
                let drop = (self.skip_frames as usize).min(out.len() / ch);
                out.drain(..drop * ch);
                self.skip_frames -= drop as u64;
                if out.is_empty() {
                    continue;
                }
            }
            return Ok(true);
        }
    }
}

/// Copia intercalada, adaptando los canales si el flujo los cambia a mitad (raro en MP3).
fn copy_out(slot: &mut Option<SampleBuffer<f32>>, channels: u16, decoded: AudioBufferRef<'_>, out: &mut Vec<f32>) {
    let spec = *decoded.spec();
    let cap = decoded.capacity() as u64;
    let buf = match slot {
        Some(b) if b.capacity() >= cap as usize * spec.channels.count() => b,
        slot => slot.insert(SampleBuffer::new(cap, spec)),
    };
    buf.copy_interleaved_ref(decoded);
    let src = buf.samples();
    let (from, to) = (spec.channels.count().max(1), usize::from(channels.max(1)));
    if from == to {
        out.extend_from_slice(src);
    } else {
        for frame in src.chunks_exact(from) {
            let mono = frame.iter().sum::<f32>() / from as f32;
            for c in 0..to {
                out.push(if from == 1 { frame[0] } else if to == 1 { mono } else { frame.get(c).copied().unwrap_or(mono) });
            }
        }
    }
}

impl AudioStream for SymphoniaStream {
    fn sample_rate(&self) -> u32 {
        self.rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn duration(&self) -> Option<Duration> {
        self.duration
    }

    fn next_chunk(&mut self, out: &mut Vec<f32>) -> Result<bool, AudioError> {
        out.clear();
        if let Some(p) = self.pending.take() {
            *out = p;
            return Ok(true);
        }
        self.decode_next(out, false)
    }

    fn seek(&mut self, to: Duration) -> Result<(), AudioError> {
        let to = self.duration.map_or(to, |d| to.min(d));
        let time = Time::new(to.as_secs(), f64::from(to.subsec_nanos()) / 1e9);
        self.pending = None;
        let seeked = self.format.seek(SeekMode::Accurate, SeekTo::Time { time, track_id: Some(self.track) }).map_err(sym_err)?;
        self.decoder.reset();
        let tb = self.format.tracks().iter().find(|t| t.id == self.track).and_then(|t| t.codec_params.time_base);
        let gap = seeked.required_ts.saturating_sub(seeked.actual_ts);
        self.skip_frames = match tb {
            Some(tb) => {
                let t = tb.calc_time(gap);
                ((t.seconds as f64 + t.frac) * f64::from(self.rate)).round() as u64
            }
            None => gap,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Arc<[u8]> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
        std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())).into()
    }

    /// WAV PCM de 16 bits generado aquí mismo.
    pub(crate) fn wav(rate: u32, channels: u16, secs: f32, freq: f32) -> Vec<u8> {
        let frames = (rate as f32 * secs) as u32;
        let data_len = frames * u32::from(channels) * 2;
        let mut v = Vec::with_capacity(44 + data_len as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + data_len).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&rate.to_le_bytes());
        v.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        v.extend_from_slice(&(channels * 2).to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_len.to_le_bytes());
        for i in 0..frames {
            let s = ((i as f32 / rate as f32 * freq * std::f32::consts::TAU).sin() * 0.5 * 32767.0) as i16;
            for _ in 0..channels {
                v.extend_from_slice(&s.to_le_bytes());
            }
        }
        v
    }

    /// Frecuencia dominante aproximada por cruces por cero del primer canal (saltando el arranque).
    fn freq(p: &Pcm) -> f32 {
        let ch = usize::from(p.channels);
        let first: Vec<f32> = p.samples.iter().step_by(ch).copied().collect();
        let skip = first.len() / 10;
        let body = &first[skip..first.len() - skip];
        let crossings = body.windows(2).filter(|w| (w[0] < 0.0) != (w[1] < 0.0)).count();
        crossings as f32 / 2.0 / (body.len() as f32 / p.sample_rate as f32)
    }

    fn rms(p: &Pcm) -> f32 {
        (p.samples.iter().map(|s| s * s).sum::<f32>() / p.samples.len().max(1) as f32).sqrt()
    }

    fn check_tone(name: &str, p: &Pcm, rate: u32, channels: u16) {
        assert_eq!((p.sample_rate, p.channels), (rate, channels), "{name}: formato");
        let d = p.duration().as_secs_f32();
        assert!((0.9..=1.15).contains(&d), "{name}: duración {d}");
        let f = freq(p);
        assert!((400.0..=480.0).contains(&f), "{name}: frecuencia {f}");
        assert!(rms(p) > 0.05, "{name}: silencio");
    }

    #[test]
    fn decodes_telegram_style_voice_note() {
        let bytes = fixture("voice.ogg");
        let s = open(bytes.clone(), Some("audio/ogg")).unwrap();
        let d = s.duration().unwrap().as_secs_f32();
        assert!((0.99..=1.01).contains(&d), "duración por granule {d}");
        let p = decode_all(bytes, None).unwrap();
        check_tone("voice.ogg", &p, 48_000, 1);
        // El pre_skip se descarta y el final se recorta a la posición granular: 1 s exacto.
        assert_eq!(p.frames(), 48_000);
    }

    #[test]
    fn decodes_stereo_opus() {
        check_tone("stereo.opus", &decode_all(fixture("stereo.opus"), None).unwrap(), 48_000, 2);
    }

    #[test]
    fn decodes_music_formats() {
        for (name, mime, rate) in [
            ("tone.mp3", Some("audio/mpeg"), 22_050),
            ("tone.m4a", Some("audio/mp4"), 22_050),
            ("tone.flac", None, 8_000),
            ("tone.ogg", Some("audio/ogg"), 22_050),
        ] {
            let p = decode_all(fixture(name), mime).unwrap_or_else(|e| panic!("{name}: {e}"));
            check_tone(name, &p, rate, 1);
        }
        let p = decode_all(wav(16_000, 2, 1.0, 440.0).into(), Some("audio/wav")).unwrap();
        check_tone("wav", &p, 16_000, 2);
        assert_eq!(p.frames(), 16_000);
    }

    #[test]
    fn seeks_in_opus_and_symphonia() {
        for name in ["voice.ogg", "tone.mp3", "tone.flac"] {
            let mut s = open(fixture(name), None).unwrap();
            let total = decode_all(fixture(name), None).unwrap().frames() as f64 / f64::from(s.sample_rate());
            s.seek(Duration::from_millis(500)).unwrap();
            let (mut n, mut chunk) = (0usize, Vec::new());
            while s.next_chunk(&mut chunk).unwrap() {
                n += chunk.len() / usize::from(s.channels());
            }
            let rest = n as f64 / f64::from(s.sample_rate());
            assert!((rest - (total - 0.5)).abs() < 0.06, "{name}: quedan {rest} s de {total} s");
        }
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert_eq!(open(Arc::from(&b"definitely not audio"[..]), None).err(), Some(AudioError::Unsupported));
        let mut cut = fixture("voice.ogg").to_vec();
        cut.truncate(cut.len() / 2);
        let p = decode_all(cut.into(), None).unwrap();
        assert!(p.duration() < Duration::from_millis(700), "lo truncado se reproduce hasta donde llega");
        // OGG/Opus con el audio destrozado: errores controlados.
        let mut bad = fixture("voice.ogg").to_vec();
        for b in bad.iter_mut().skip(200) {
            *b = b.wrapping_mul(31).wrapping_add(7);
        }
        let _ = decode_all(bad.into(), None);
    }
}
