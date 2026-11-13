//! Solo pruebas: multiplexor MP4 mínimo (moov al final, como un MP4 sin «faststart») y utilidades
//! para fabricar vídeos H.264 de prueba con el codificador de OpenH264, sin ffmpeg.

/// Caja con su cabecera.
pub fn fbox(typ: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
    v.extend_from_slice(typ);
    v.extend_from_slice(body);
    v
}

fn full(typ: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut b = vec![0, 0, 0, 0];
    b.extend_from_slice(body);
    fbox(typ, &b)
}

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_be_bytes()).collect()
}

/// Muestra ya en formato AVCC (NAL con prefijo de 4 bytes).
pub struct MuxSample {
    pub avcc: Vec<u8>,
    pub duration: u32,
    pub cts: i32,
}

/// MP4 con una pista `avc1`: `ftyp`, `mdat` y `moov` (en ese orden). `per_chunk` muestras por trozo.
pub fn mux(width: u16, height: u16, timescale: u32, sps: &[Vec<u8>], pps: &[Vec<u8>], samples: &[MuxSample], per_chunk: usize) -> Vec<u8> {
    let ftyp = fbox(b"ftyp", b"isom\0\0\x02\0isomavc1");
    let data: Vec<u8> = samples.iter().flat_map(|s| s.avcc.clone()).collect();
    let mdat = fbox(b"mdat", &data);
    let base = (ftyp.len() + 8) as u32;

    let mut chunks = Vec::new();
    let mut at = base;
    for (i, s) in samples.iter().enumerate() {
        if i % per_chunk == 0 {
            chunks.push(at);
        }
        at += s.avcc.len() as u32;
    }

    let mut avcc = vec![1, 0x64, 0, 0x1e, 0xff, 0xe0 | sps.len() as u8];
    for p in sps {
        avcc.extend_from_slice(&(p.len() as u16).to_be_bytes());
        avcc.extend_from_slice(p);
    }
    avcc.push(pps.len() as u8);
    for p in pps {
        avcc.extend_from_slice(&(p.len() as u16).to_be_bytes());
        avcc.extend_from_slice(p);
    }
    let mut entry = vec![0u8; 6];
    entry.extend_from_slice(&1u16.to_be_bytes());
    entry.extend_from_slice(&[0; 16]);
    entry.extend_from_slice(&width.to_be_bytes());
    entry.extend_from_slice(&height.to_be_bytes());
    entry.extend_from_slice(&u32s(&[0x0048_0000, 0x0048_0000, 0]));
    entry.extend_from_slice(&1u16.to_be_bytes());
    entry.extend_from_slice(&[0; 32]);
    entry.extend_from_slice(&[0x00, 0x18, 0xff, 0xff]);
    entry.extend(fbox(b"avcC", &avcc));
    let mut stsd = u32s(&[1]);
    stsd.extend(fbox(b"avc1", &entry));

    let n = samples.len() as u32;
    let mut stts = u32s(&[n]);
    let mut ctts = u32s(&[n]);
    let mut stsz = u32s(&[0, n]);
    for s in samples {
        stts.extend(u32s(&[1, s.duration]));
        ctts.extend(u32s(&[1, s.cts as u32]));
        stsz.extend(u32s(&[s.avcc.len() as u32]));
    }
    let stsc = u32s(&[1, 1, per_chunk as u32, 1]);
    let mut stco = u32s(&[chunks.len() as u32]);
    stco.extend(u32s(&chunks));

    let stbl = [full(b"stsd", &stsd), full(b"stts", &stts), full(b"ctts", &ctts), full(b"stsz", &stsz), full(b"stsc", &stsc), full(b"stco", &stco)].concat();
    let minf = fbox(b"minf", &fbox(b"stbl", &stbl));
    let duration: u32 = samples.iter().map(|s| s.duration).sum();
    let mdhd = full(b"mdhd", &u32s(&[0, 0, timescale, duration, 0x55c4_0000]));
    let hdlr = full(b"hdlr", &[&u32s(&[0])[..], b"vide", &[0; 13]].concat());
    let trak = fbox(b"trak", &fbox(b"mdia", &[mdhd, hdlr, minf].concat()));
    let moov = fbox(b"moov", &[full(b"mvhd", &[0; 96]), trak].concat());
    [ftyp, mdat, moov].concat()
}

/// Parte un flujo Annex B en NAL (sin los códigos de inicio).
pub fn split_annex_b(stream: &[u8]) -> Vec<&[u8]> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 <= stream.len() {
        if stream[i..i + 3] == [0, 0, 1] {
            starts.push(i + 3);
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut out = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let mut e = starts.get(k + 1).map_or(stream.len(), |&n| n - 3);
        while e > s && stream[e - 1] == 0 && starts.get(k + 1).is_some() {
            e -= 1;
        }
        out.push(&stream[s..e]);
    }
    out
}

/// Codifica `frames` (RGB) con OpenH264 y los mete en un MP4 (`ms` por fotograma, timescale 1000).
pub fn encode_mp4(w: u16, h: u16, frames: &[Vec<u8>], ms: u32) -> Vec<u8> {
    use openh264::encoder::Encoder;
    use openh264::formats::{RgbSliceU8, YUVBuffer};
    let mut enc = Encoder::new().expect("codificador");
    let (mut sps, mut pps, mut samples) = (Vec::new(), Vec::new(), Vec::new());
    for rgb in frames {
        let yuv = YUVBuffer::from_rgb_source(RgbSliceU8::new(rgb, (usize::from(w), usize::from(h))));
        let stream = enc.encode(&yuv).expect("codificar").to_vec();
        let mut avcc = Vec::new();
        for nal in split_annex_b(&stream) {
            match nal.first().map(|b| b & 0x1f) {
                Some(7) => sps = vec![nal.to_vec()],
                Some(8) => pps = vec![nal.to_vec()],
                Some(_) => {
                    avcc.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                    avcc.extend_from_slice(nal);
                }
                None => {}
            }
        }
        samples.push(MuxSample { avcc, duration: ms, cts: 0 });
    }
    mux(w, h, 1000, &sps, &pps, &samples, 3)
}

/// Fotograma RGB de un color liso.
pub fn solid(w: u16, h: u16, rgb: [u8; 3]) -> Vec<u8> {
    rgb.iter().copied().cycle().take(usize::from(w) * usize::from(h) * 3).collect()
}
