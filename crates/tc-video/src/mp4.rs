//! Demultiplexor MP4 (ISO BMFF) mínimo: solo lo que hace falta para la primera pista de vídeo H.264
//! de un MP4 «normal» (no fragmentado), que es como Telegram guarda los GIF.
//!
//! Lee `moov/trak/mdia/{mdhd,hdlr,minf/stbl/{stsd(avc1|avc3 → avcC),stts,ctts,stsz,stsc,stco|co64}}`.
//! Todo acceso a los bytes está acotado: un archivo dañado da un error, nunca un pánico ni una
//! reserva desmesurada.

use crate::VideoError;

fn bad(what: &str) -> VideoError {
    VideoError::Container(what.to_string())
}

fn be16(d: &[u8], at: usize) -> Result<u16, VideoError> {
    d.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]])).ok_or_else(|| bad("caja truncada"))
}

fn be32(d: &[u8], at: usize) -> Result<u32, VideoError> {
    d.get(at..at + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]])).ok_or_else(|| bad("caja truncada"))
}

fn be64(d: &[u8], at: usize) -> Result<u64, VideoError> {
    d.get(at..at + 8).map(|b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])).ok_or_else(|| bad("caja truncada"))
}

/// Recorre las cajas hijas de `data`: `(tipo, cuerpo, desplazamiento del cuerpo dentro de data)`.
/// Una caja que dice ser más larga que lo que queda se recorta (MP4 truncado: el `moov` del
/// principio sigue sirviendo).
struct Boxes<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for Boxes<'a> {
    type Item = ([u8; 4], &'a [u8], usize);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = self.data.get(self.pos..)?;
        if rest.len() < 8 {
            return None;
        }
        let size32 = u32::from_be_bytes([rest[0], rest[1], rest[2], rest[3]]) as u64;
        let typ = [rest[4], rest[5], rest[6], rest[7]];
        let (header, size) = match size32 {
            0 => (8, rest.len() as u64),
            1 => (16, be64(rest, 8).ok()?),
            n => (8, n),
        };
        if size < header as u64 {
            return None;
        }
        let end = usize::try_from(size).unwrap_or(usize::MAX).min(rest.len());
        let body = rest.get(header..end)?;
        let at = self.pos + header;
        self.pos += end;
        Some((typ, body, at))
    }
}

fn boxes(data: &[u8]) -> Boxes<'_> {
    Boxes { data, pos: 0 }
}

fn child<'a>(data: &'a [u8], typ: &[u8; 4]) -> Option<&'a [u8]> {
    boxes(data).find(|(t, ..)| t == typ).map(|(_, b, _)| b)
}

fn need<'a>(data: &'a [u8], typ: &[u8; 4]) -> Result<&'a [u8], VideoError> {
    child(data, typ).ok_or_else(|| bad(&format!("falta la caja «{}»", String::from_utf8_lossy(typ))))
}

/// Una muestra (= un fotograma, en orden de decodificación).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    /// Posición y tamaño dentro del archivo.
    pub offset: usize,
    pub size: usize,
    /// Instante de decodificación y duración (unidades de `Track::timescale`).
    pub dts: u64,
    pub duration: u32,
    /// Desplazamiento de composición (`ctts`): presentación = `dts + cts`.
    pub cts: i64,
}

/// Pista H.264 lista para decodificar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub width: u32,
    pub height: u32,
    pub timescale: u32,
    /// Bytes del prefijo de longitud de cada NAL (1, 2 o 4).
    pub length_size: usize,
    pub sps: Vec<Vec<u8>>,
    pub pps: Vec<Vec<u8>>,
    pub samples: Vec<Sample>,
}

impl Track {
    /// Bytes de la muestra `s` dentro del archivo.
    pub fn data<'a>(&self, file: &'a [u8], s: &Sample) -> Result<&'a [u8], VideoError> {
        file.get(s.offset..s.offset.saturating_add(s.size)).ok_or_else(|| bad("muestra fuera del archivo"))
    }
}

/// ¿Parece un MP4/MOV? (primera caja conocida de ISO BMFF).
pub fn looks_like_mp4(bytes: &[u8]) -> bool {
    matches!(bytes.get(4..8), Some(b"ftyp" | b"moov" | b"mdat" | b"free" | b"skip" | b"wide"))
}

/// Primera pista de vídeo H.264 del MP4.
pub fn parse(file: &[u8]) -> Result<Track, VideoError> {
    let moov = need(file, b"moov")?;
    let mut other_codec = None;
    for (typ, trak, _) in boxes(moov) {
        if &typ != b"trak" {
            continue;
        }
        match parse_trak(file, trak)? {
            Trak::Avc(t) => return Ok(t),
            Trak::OtherCodec(c) => other_codec = Some(c),
            Trak::NotVideo => {}
        }
    }
    Err(match other_codec {
        Some(c) => VideoError::Unsupported(c),
        None => VideoError::NoVideo,
    })
}

enum Trak {
    Avc(Track),
    OtherCodec(String),
    NotVideo,
}

fn parse_trak(file: &[u8], trak: &[u8]) -> Result<Trak, VideoError> {
    let mdia = need(trak, b"mdia")?;
    let hdlr = need(mdia, b"hdlr")?;
    if hdlr.get(8..12) != Some(b"vide") {
        return Ok(Trak::NotVideo);
    }
    let mdhd = need(mdia, b"mdhd")?;
    let timescale = if mdhd.first() == Some(&1) { be32(mdhd, 20)? } else { be32(mdhd, 12)? };
    let stbl = need(need(mdia, b"minf")?, b"stbl")?;

    // stsd: versión/flags (4) + número de entradas (4) + entradas (cajas).
    let stsd = need(stbl, b"stsd")?;
    let Some((codec, entry, _)) = boxes(stsd.get(8..).unwrap_or_default()).next() else {
        return Err(bad("stsd sin entradas"));
    };
    if &codec != b"avc1" && &codec != b"avc3" {
        return Ok(Trak::OtherCodec(String::from_utf8_lossy(&codec).into_owned()));
    }
    // VisualSampleEntry: 6 reservados + índice (2) + 16 predefinidos + ancho (2) + alto (2) + … = 78 bytes.
    let (width, height) = (u32::from(be16(entry, 24)?), u32::from(be16(entry, 26)?));
    let avcc = need(entry.get(78..).unwrap_or_default(), b"avcC")?;
    let (length_size, sps, pps) = parse_avcc(avcc)?;

    let samples = sample_table(file, stbl)?;
    if samples.is_empty() {
        // Los MP4 fragmentados (`moof`) tienen la tabla vacía: no se leen.
        return Err(bad("la pista no tiene muestras (¿MP4 fragmentado?)"));
    }
    Ok(Trak::Avc(Track { width, height, timescale, length_size, sps, pps, samples }))
}

type ParamSets = (usize, Vec<Vec<u8>>, Vec<Vec<u8>>);

fn parse_avcc(d: &[u8]) -> Result<ParamSets, VideoError> {
    let length_size = usize::from(d.get(4).ok_or_else(|| bad("avcC truncado"))? & 0b11) + 1;
    if length_size == 3 {
        return Err(bad("avcC con prefijos de 3 bytes"));
    }
    let mut at = 5;
    let read_sets = |count: usize, at: &mut usize| -> Result<Vec<Vec<u8>>, VideoError> {
        let mut v = Vec::with_capacity(count);
        for _ in 0..count {
            let n = usize::from(be16(d, *at)?);
            let ps = d.get(*at + 2..*at + 2 + n).ok_or_else(|| bad("avcC truncado"))?;
            v.push(ps.to_vec());
            *at += 2 + n;
        }
        Ok(v)
    };
    let n_sps = usize::from(d.get(at).ok_or_else(|| bad("avcC truncado"))? & 0x1f);
    at += 1;
    let sps = read_sets(n_sps, &mut at)?;
    let n_pps = usize::from(*d.get(at).ok_or_else(|| bad("avcC truncado"))?);
    at += 1;
    let pps = read_sets(n_pps, &mut at)?;
    Ok((length_size, sps, pps))
}

/// Número de entradas de una tabla `full box` (tras versión/flags), acotado por lo que cabe.
fn entries(d: &[u8], at: usize, entry_bytes: usize) -> Result<usize, VideoError> {
    let n = be32(d, at)? as usize;
    let room = d.len().saturating_sub(at + 4) / entry_bytes.max(1);
    if n > room {
        return Err(bad("tabla más larga que su caja"));
    }
    Ok(n)
}

fn sample_table(file: &[u8], stbl: &[u8]) -> Result<Vec<Sample>, VideoError> {
    // stsz: tamaño fijo o uno por muestra.
    let stsz = need(stbl, b"stsz")?;
    let fixed = be32(stsz, 4)? as usize;
    let count = be32(stsz, 8)? as usize;
    // Con tamaño fijo, cada muestra ocupa al menos `fixed` bytes del archivo: así el número queda acotado.
    let sizes: Vec<usize> = match file.len().checked_div(fixed) {
        None => {
            let n = entries(stsz, 8, 4)?;
            (0..n).map(|i| be32(stsz, 12 + 4 * i).map(|s| s as usize)).collect::<Result<_, _>>()?
        }
        Some(room) if count > room => return Err(bad("stsz con más muestras que bytes")),
        Some(_) => vec![fixed; count],
    };
    let count = sizes.len();

    // stco/co64: posición de cada trozo.
    let chunks: Vec<u64> = if let Some(stco) = child(stbl, b"stco") {
        let n = entries(stco, 4, 4)?;
        (0..n).map(|i| be32(stco, 8 + 4 * i).map(u64::from)).collect::<Result<_, _>>()?
    } else {
        let co64 = need(stbl, b"co64")?;
        let n = entries(co64, 4, 8)?;
        (0..n).map(|i| be64(co64, 8 + 8 * i)).collect::<Result<_, _>>()?
    };

    // stsc: (primer trozo, muestras por trozo) por tramos.
    let stsc = need(stbl, b"stsc")?;
    let n = entries(stsc, 4, 12)?;
    let runs: Vec<(u32, u32)> = (0..n).map(|i| Ok((be32(stsc, 8 + 12 * i)?, be32(stsc, 12 + 12 * i)?))).collect::<Result<_, VideoError>>()?;
    if runs.is_empty() {
        return Err(bad("stsc vacío"));
    }

    let mut samples = Vec::with_capacity(count);
    let mut run = 0;
    'chunks: for (ci, &chunk_at) in chunks.iter().enumerate() {
        let chunk_no = ci as u32 + 1;
        while run + 1 < runs.len() && runs[run + 1].0 <= chunk_no {
            run += 1;
        }
        let mut at = usize::try_from(chunk_at).map_err(|_| bad("trozo fuera del archivo"))?;
        for _ in 0..runs[run].1 {
            let Some(&size) = sizes.get(samples.len()) else { break 'chunks };
            samples.push(Sample { offset: at, size, dts: 0, duration: 0, cts: 0 });
            at = at.saturating_add(size);
        }
    }

    // stts: duraciones (y con ellas el dts).
    let stts = need(stbl, b"stts")?;
    let n = entries(stts, 4, 8)?;
    let mut i = 0;
    let mut dts = 0u64;
    for e in 0..n {
        let (k, delta) = (be32(stts, 8 + 8 * e)?, be32(stts, 12 + 8 * e)?);
        for _ in 0..k {
            let Some(s) = samples.get_mut(i) else { break };
            s.dts = dts;
            s.duration = delta;
            dts += u64::from(delta);
            i += 1;
        }
    }

    // ctts (opcional, con fotogramas B): desplazamiento de composición.
    if let Some(ctts) = child(stbl, b"ctts") {
        let n = entries(ctts, 4, 8)?;
        let mut i = 0;
        for e in 0..n {
            let (k, off) = (be32(ctts, 8 + 8 * e)?, be32(ctts, 12 + 8 * e)? as i32);
            for _ in 0..k {
                let Some(s) = samples.get_mut(i) else { break };
                s.cts = i64::from(off);
                i += 1;
            }
        }
    }
    Ok(samples)
}

/// Pasa una muestra AVCC (NAL con prefijo de longitud) a Annex B (códigos de inicio) al final de `out`.
pub fn annex_b(sample: &[u8], length_size: usize, out: &mut Vec<u8>) -> Result<(), VideoError> {
    let mut p = sample;
    while !p.is_empty() {
        let len_bytes = p.get(..length_size).ok_or_else(|| bad("NAL truncada"))?;
        let n = len_bytes.iter().fold(0usize, |acc, b| (acc << 8) | usize::from(*b));
        p = &p[length_size..];
        let nal = p.get(..n).ok_or_else(|| bad("NAL más larga que la muestra"))?;
        if !nal.is_empty() {
            out.extend_from_slice(&[0, 0, 0, 1]);
            out.extend_from_slice(nal);
        }
        p = &p[n..];
    }
    Ok(())
}

/// Duración en ms de cada fotograma en orden de presentación (el que devuelve el decodificador).
pub fn display_durations_ms(track: &Track) -> Vec<u32> {
    let scale = u64::from(track.timescale);
    let mut pts: Vec<(i64, u32)> = track.samples.iter().map(|s| (s.dts as i64 + s.cts, s.duration)).collect();
    pts.sort_by_key(|p| p.0);
    let to_ms = |t: u64| t.saturating_mul(1000).saturating_add(scale / 2).checked_div(scale).map_or(40, |ms| ms.clamp(1, u64::from(u32::MAX)) as u32);
    (0..pts.len())
        .map(|k| {
            let d = match pts.get(k + 1) {
                Some(next) if next.0 > pts[k].0 => (next.0 - pts[k].0) as u64,
                _ => u64::from(pts[k].1),
            };
            if d == 0 { 40 } else { to_ms(d) }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testmux::{MuxSample, fbox, mux};

    fn sample(bytes: &[u8], duration: u32, cts: i32) -> MuxSample {
        MuxSample { avcc: bytes.to_vec(), duration, cts }
    }

    #[test]
    fn lee_la_tabla_de_muestras_y_los_parametros() {
        // Dos trozos (stsc: 2 + 1 muestras), fotogramas B (ctts) y timescale 1000.
        let samples = [sample(&[0, 0, 0, 2, 0x65, 1], 100, 100), sample(&[0, 0, 0, 1, 0x41], 100, 300), sample(&[0, 0, 0, 1, 0x01], 100, 0)];
        let file = mux(64, 48, 1000, &[vec![0x67, 1, 2]], &[vec![0x68, 3]], &samples, 2);
        assert!(looks_like_mp4(&file));
        let t = parse(&file).expect("MP4 válido");
        assert_eq!((t.width, t.height, t.timescale, t.length_size), (64, 48, 1000, 4));
        assert_eq!((t.sps.clone(), t.pps.clone()), (vec![vec![0x67, 1, 2]], vec![vec![0x68, 3]]));
        assert_eq!(t.samples.len(), 3);
        assert_eq!(t.samples.iter().map(|s| (s.dts, s.cts)).collect::<Vec<_>>(), [(0, 100), (100, 300), (200, 0)]);
        assert_eq!(t.data(&file, &t.samples[1]).unwrap(), &[0, 0, 0, 1, 0x41]);
        assert_eq!(t.data(&file, &t.samples[2]).unwrap(), &[0, 0, 0, 1, 0x01]);
        // Presentación: 100 (s0), 200 (s2), 400 (s1) → 100 ms, 200 ms y la duración de la última.
        assert_eq!(display_durations_ms(&t), [100, 200, 100]);
    }

    #[test]
    fn annex_b_pone_codigos_de_inicio() {
        let mut out = Vec::new();
        annex_b(&[0, 0, 0, 2, 0x65, 9, 0, 0, 0, 1, 0x06], 4, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 1, 0x65, 9, 0, 0, 0, 1, 0x06]);
        out.clear();
        annex_b(&[0, 2, 0x41, 7], 2, &mut out).unwrap();
        assert_eq!(out, [0, 0, 0, 1, 0x41, 7]);
        assert!(annex_b(&[0, 0, 0, 9, 1], 4, &mut out).is_err(), "NAL más larga que la muestra");
    }

    #[test]
    fn archivos_rotos_dan_error_sin_panico() {
        assert!(!looks_like_mp4(b"GIF89a....."));
        assert!(matches!(parse(b"no es un mp4"), Err(VideoError::Container(_))));
        let good = mux(8, 8, 1000, &[vec![0x67]], &[vec![0x68]], &[sample(&[0, 0, 0, 1, 0x65], 40, 0)], 1);
        // Truncar en cualquier punto no debe entrar en pánico.
        for cut in 0..good.len() {
            let _ = parse(&good[..cut]);
        }
        // Sin pista de vídeo.
        let audio_only = fbox(b"moov", &fbox(b"trak", &fbox(b"mdia", &fbox(b"hdlr", &[0, 0, 0, 0, 0, 0, 0, 0, b's', b'o', b'u', b'n']))));
        assert_eq!(parse(&audio_only), Err(VideoError::NoVideo));
        // Número de entradas que no cabe en su caja.
        let mut evil = good.clone();
        let at = evil.windows(4).position(|w| w == b"stsz").unwrap() + 8;
        evil[at + 4..at + 8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert!(parse(&evil).is_err());
    }
}
