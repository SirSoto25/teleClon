//! Forma de onda de las notas de voz de Telegram (`DocumentAttributeAudio.waveform`).
//!
//! Son valores de 5 bits (0..=31) empaquetados de forma contigua en little-endian: el valor `i`
//! empieza en el bit `5 * i` del flujo, contando desde el bit menos significativo del primer byte.
//! Con `n` bytes caben `n * 8 / 5` valores (normalmente 100 en 63 bytes).

/// Valor máximo de una muestra.
pub const MAX: u8 = 31;

/// Desempaqueta la forma de onda (valores 0..=31). Los bits sobrantes del final se ignoran.
pub fn decode(packed: &[u8]) -> Vec<u8> {
    let count = packed.len() * 8 / 5;
    (0..count)
        .map(|i| {
            let bit = i * 5;
            let (byte, shift) = (bit / 8, bit % 8);
            let lo = u16::from(packed[byte]);
            let hi = packed.get(byte + 1).copied().map_or(0, u16::from);
            (((hi << 8) | lo) >> shift) as u8 & MAX
        })
        .collect()
}

/// Empaqueta valores (se recortan a 0..=31). Inversa de [`decode`].
pub fn encode(values: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; (values.len() * 5).div_ceil(8)];
    for (i, &v) in values.iter().enumerate() {
        let v = u16::from(v.min(MAX));
        let bit = i * 5;
        let (byte, shift) = (bit / 8, bit % 8);
        let w = v << shift;
        out[byte] |= w as u8;
        if let Some(b) = out.get_mut(byte + 1) {
            *b |= (w >> 8) as u8;
        }
    }
    out
}

/// Reescala a `n` barras tomando el máximo de cada tramo (o repitiendo si hay menos valores).
/// Vacío si no hay valores o `n == 0`.
pub fn resample(values: &[u8], n: usize) -> Vec<u8> {
    if values.is_empty() || n == 0 {
        return Vec::new();
    }
    (0..n)
        .map(|i| {
            let a = i * values.len() / n;
            let b = ((i + 1) * values.len() / n).max(a + 1).min(values.len());
            values[a..b].iter().copied().max().unwrap_or(0)
        })
        .collect()
}

/// Forma de onda calculada a partir de muestras intercaladas (pico por tramo, escala 0..=31).
pub fn from_samples(samples: &[f32], channels: usize, n: usize) -> Vec<u8> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    if frames == 0 || n == 0 {
        return Vec::new();
    }
    let peaks: Vec<f32> = (0..n)
        .map(|i| {
            let a = i * frames / n;
            let b = ((i + 1) * frames / n).max(a + 1).min(frames);
            samples[a * channels..b * channels].iter().fold(0.0f32, |m, s| m.max(s.abs()))
        })
        .collect();
    let top = peaks.iter().copied().fold(0.0f32, f32::max);
    if top <= f32::EPSILON {
        return vec![0; n];
    }
    peaks.iter().map(|p| ((p / top) * f32::from(MAX)).round() as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_bit_layout() {
        // 31 en los bits 0-4, 0 en 5-9, 1 en 10-14 (bit 10 = byte 1, bit 2) y 2 en 15-19 (bit 16 =
        // byte 2, bit 0).
        assert_eq!(decode(&[0x1F, 0x04, 0x01]), vec![31, 0, 1, 2]);
        // Telegram envía 100 valores en 63 bytes.
        let vals: Vec<u8> = (0..100).map(|i| (i * 7 % 32) as u8).collect();
        let packed = encode(&vals);
        assert_eq!(packed.len(), 63);
        assert_eq!(&decode(&packed)[..100], vals.as_slice());
    }

    #[test]
    fn roundtrip_any_length() {
        for n in 0..40usize {
            let vals: Vec<u8> = (0..n).map(|i| (i * 13 % 32) as u8).collect();
            let back = decode(&encode(&vals));
            assert_eq!(&back[..n], vals.as_slice(), "n = {n}");
            assert!(back.len() - n < 2, "como mucho un valor de relleno");
        }
    }

    #[test]
    fn resample_and_from_samples() {
        assert_eq!(resample(&[1, 5, 2, 9], 2), vec![5, 9]);
        assert_eq!(resample(&[3, 7], 4), vec![3, 3, 7, 7]);
        assert!(resample(&[], 4).is_empty());
        let s = [0.0, 0.5, -1.0, 0.25];
        assert_eq!(from_samples(&s, 1, 2), vec![16, 31]);
        assert_eq!(from_samples(&[0.0; 8], 2, 2), vec![0, 0]);
    }
}
