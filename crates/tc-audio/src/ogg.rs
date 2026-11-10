//! Demultiplexor Ogg mínimo para OGG/Opus (RFC 3533 + RFC 7845): solo el primer flujo lógico, sin
//! comprobar el CRC (un archivo dañado se nota al decodificar). Basta para las notas de voz.
use crate::AudioError;

/// Muestras por segundo a las que trabaja Opus internamente (y en las que van las posiciones).
pub(crate) const OPUS_RATE: u32 = 48_000;

/// Cabecera `OpusHead`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpusHead {
    pub channels: u8,
    pub pre_skip: u16,
    /// Ganancia de salida en dB, formato Q7.8.
    pub gain_q8: i16,
    pub mapping_family: u8,
}

/// Flujo Opus ya separado en paquetes de audio.
#[derive(Debug)]
pub(crate) struct OggOpus {
    pub head: OpusHead,
    pub packets: Vec<Vec<u8>>,
    /// Posición granular (48 kHz, incluye `pre_skip`) de la última página con paquetes, si la hay.
    pub last_granule: Option<u64>,
}

fn corrupt(what: &str) -> AudioError {
    AudioError::Corrupt(what.into())
}

/// ¿Empieza como un OGG cuyo primer paquete es `OpusHead`?
pub(crate) fn is_ogg_opus(b: &[u8]) -> bool {
    if b.len() < 28 || &b[..4] != b"OggS" {
        return false;
    }
    let nsegs = b[26] as usize;
    b.get(27 + nsegs..27 + nsegs + 8).is_some_and(|m| m == b"OpusHead")
}

/// Muestras (a 48 kHz, por canal) de un paquete Opus según su TOC (RFC 6716 §3.1). 0 si no es válido.
pub(crate) fn packet_samples(p: &[u8]) -> u64 {
    let Some(&toc) = p.first() else { return 0 };
    let config = toc >> 3;
    let frame: u64 = match config {
        0..=11 => [480, 960, 1920, 2880][usize::from(config % 4)],
        12..=15 => [480, 960][usize::from(config % 2)],
        _ => [120, 240, 480, 960][usize::from(config % 4)],
    };
    let frames = match toc & 3 {
        0 => 1,
        1 | 2 => 2,
        _ => p.get(1).map_or(0, |c| u64::from(c & 0x3F)),
    };
    frame * frames
}

/// Separa el archivo en cabecera y paquetes de audio.
pub(crate) fn parse(b: &[u8]) -> Result<OggOpus, AudioError> {
    let mut pos = 0usize;
    let mut serial = None;
    let mut packets: Vec<Vec<u8>> = Vec::new();
    let mut partial: Vec<u8> = Vec::new();
    let mut last_granule = None;
    while pos + 27 <= b.len() {
        if &b[pos..pos + 4] != b"OggS" {
            // Basura entre páginas: se busca la siguiente captura.
            match b[pos + 1..].windows(4).position(|w| w == b"OggS") {
                Some(off) => {
                    pos += 1 + off;
                    continue;
                }
                None => break,
            }
        }
        let granule = i64::from_le_bytes(b[pos + 6..pos + 14].try_into().map_err(|_| corrupt("página"))?);
        let page_serial = u32::from_le_bytes(b[pos + 14..pos + 18].try_into().map_err(|_| corrupt("página"))?);
        let nsegs = b[pos + 26] as usize;
        let lacing = b.get(pos + 27..pos + 27 + nsegs).ok_or_else(|| corrupt("página truncada"))?;
        let mut data = pos + 27 + nsegs;
        let body: usize = lacing.iter().map(|&l| l as usize).sum();
        let end = data + body;
        if end > b.len() {
            break; // Página final truncada: se usa lo que haya.
        }
        if *serial.get_or_insert(page_serial) == page_serial {
            let mut completed = false;
            for &l in lacing {
                partial.extend_from_slice(&b[data..data + l as usize]);
                data += l as usize;
                if l < 255 {
                    packets.push(std::mem::take(&mut partial));
                    completed = true;
                }
            }
            if completed && granule >= 0 && packets.len() > 2 {
                last_granule = Some(granule as u64);
            }
        }
        pos = end;
    }
    let mut it = packets.into_iter();
    let head = it.next().ok_or_else(|| corrupt("sin cabecera"))?;
    if head.len() < 19 || &head[..8] != b"OpusHead" {
        return Err(AudioError::Unsupported);
    }
    let head = OpusHead {
        channels: head[9],
        pre_skip: u16::from_le_bytes([head[10], head[11]]),
        gain_q8: i16::from_le_bytes([head[16], head[17]]),
        mapping_family: head[18],
    };
    if head.channels == 0 {
        return Err(corrupt("cabecera"));
    }
    // El segundo paquete es `OpusTags` (metadatos): no se usa.
    let packets: Vec<Vec<u8>> = it.skip(1).filter(|p| !p.is_empty()).collect();
    Ok(OggOpus { head, packets, last_granule })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toc_durations() {
        assert_eq!(packet_samples(&[0b0000_1000]), 960); // SILK 20 ms, un frame
        assert_eq!(packet_samples(&[(31 << 3) | 1]), 1920); // CELT 20 ms, dos frames
        assert_eq!(packet_samples(&[(16 << 3) | 3, 3]), 360); // CELT 2,5 ms ×3
        assert_eq!(packet_samples(&[]), 0);
    }

    #[test]
    fn rejects_non_opus() {
        assert!(!is_ogg_opus(b"ID3\x04garbage"));
        assert!(parse(b"nada").is_err());
    }
}
