//! Notas de voz sintéticas del mock: forma de onda determinista y un WAV (8 kHz, mono, 16 bits) cuyo
//! volumen sigue esa forma de onda, para ejercitar la descarga y el reproductor sin red.

/// Barras de la forma de onda (las mismas que manda Telegram).
pub const BARS: usize = 100;
const RATE: u32 = 8_000;

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x
}

/// Forma de onda (0..=31) determinista para `seed`: frases con pausas.
pub fn waveform(seed: i64) -> Vec<u8> {
    (0..BARS)
        .map(|i| {
            let phrase = ((i as f32 / 9.0).sin().abs() * 0.8 + 0.2) * if (i / 17) % 3 == 2 { 0.25 } else { 1.0 };
            let jitter = (mix((seed as u64) ^ ((i as u64) << 20)) % 10) as f32 / 10.0;
            ((phrase * (0.6 + 0.4 * jitter)) * 31.0).round().clamp(1.0, 31.0) as u8
        })
        .collect()
}

/// Bytes de [`voice_wav`] para `secs` segundos.
pub fn wav_len(secs: u32) -> u64 {
    44 + u64::from(RATE * secs.max(1) * 2)
}

/// WAV de `secs` segundos cuya envolvente sigue [`waveform`].
pub fn voice_wav(seed: i64, secs: u32) -> Vec<u8> {
    let wave = waveform(seed);
    let frames = RATE * secs.max(1);
    let data_len = frames * 2;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&RATE.to_le_bytes());
    v.extend_from_slice(&(RATE * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    let base = 180.0 + (mix(seed as u64) % 120) as f32;
    for n in 0..frames {
        let t = n as f32 / RATE as f32;
        let bar = (n as usize * BARS / frames as usize).min(BARS - 1);
        let env = f32::from(wave[bar]) / 31.0;
        let s = (t * base * std::f32::consts::TAU).sin() * 0.6 + (t * base * 2.0 * std::f32::consts::TAU).sin() * 0.2;
        v.extend_from_slice(&((s * env * 0.5 * 32767.0) as i16).to_le_bytes());
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_well_formed() {
        assert_eq!(waveform(3), waveform(3));
        assert_ne!(waveform(3), waveform(4));
        assert!(waveform(3).iter().all(|v| (1..=31).contains(v)));
        let w = voice_wav(3, 2);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len() as u64, wav_len(2));
    }
}
