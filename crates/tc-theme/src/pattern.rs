//! Fondo del chat: curvas de nivel generativas, deterministas por id de chat.
//!
//! Un campo de ruido de valor (fBm de 3 octavas, semilla = id del chat) se muestrea en una rejilla y se
//! extraen isolíneas con marching squares. El campo se expresa en «px de valor»: el nivel `k` vale
//! `k * spacing`, así que las líneas quedan separadas `spacing` unidades de valor (28 normal, 14 en
//! chats secretos). Las coordenadas devueltas están en px lógicos del lienzo.

/// Segmento de isolínea en px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub a: (f32, f32),
    pub b: (f32, f32),
}

/// Parámetros del patrón (`pattern/*` de tokens.json).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContourParams {
    /// Separación entre niveles (28; 14 en chats secretos).
    pub spacing: f32,
    /// Frecuencia del ruido (0.004 por px).
    pub noise_scale: f32,
    /// Trazo en px.
    pub stroke: f32,
    /// Lado de celda del muestreo en px.
    pub cell: f32,
}

impl ContourParams {
    pub const DEFAULT: ContourParams = ContourParams { spacing: 28.0, noise_scale: 0.004, stroke: 1.0, cell: 8.0 };
    /// Chats secretos: doble de densidad.
    pub const SECRET: ContourParams = ContourParams { spacing: 14.0, ..Self::DEFAULT };
}

impl Default for ContourParams {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Amplitud del campo en unidades de valor (≈ 8 niveles con spacing 28).
const AMPLITUDE: f32 = 224.0;

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn lattice(seed: u64, ix: i32, iy: i32, octave: u64) -> f32 {
    let h = mix(seed ^ mix((ix as u32 as u64) << 32 | iy as u32 as u64) ^ octave.wrapping_mul(0x9e37_79b9_7f4a_7c15));
    (h >> 40) as f32 / (1u64 << 24) as f32 // [0, 1)
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(seed: u64, x: f32, y: f32, octave: u64) -> f32 {
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (smooth(x - x0), smooth(y - y0));
    let (ix, iy) = (x0 as i32, y0 as i32);
    let a = lattice(seed, ix, iy, octave);
    let b = lattice(seed, ix + 1, iy, octave);
    let c = lattice(seed, ix, iy + 1, octave);
    let d = lattice(seed, ix + 1, iy + 1, octave);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fy
}

/// Valor del campo en (x, y) px, en unidades de valor (≈ 0..AMPLITUDE).
pub fn field(chat_id: i64, x: f32, y: f32, noise_scale: f32) -> f32 {
    let seed = mix(chat_id as u64);
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..3u64 {
        sum += amp * value_noise(seed, x * noise_scale * freq, y * noise_scale * freq, o);
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm * AMPLITUDE
}

/// Isolíneas del campo del chat sobre un lienzo `width` × `height` px.
/// Determinista: mismo (chat_id, tamaño, parámetros) → mismos segmentos, en el mismo orden.
pub fn contours(chat_id: i64, width: f32, height: f32, p: ContourParams) -> Vec<Segment> {
    let mut out = Vec::new();
    if width <= 0.0 || height <= 0.0 || p.cell <= 0.0 || p.spacing <= 0.0 {
        return out;
    }
    let nx = (width / p.cell).ceil() as usize;
    let ny = (height / p.cell).ceil() as usize;
    let sample = |i: usize, j: usize| field(chat_id, i as f32 * p.cell, j as f32 * p.cell, p.noise_scale);
    let mut prev: Vec<f32> = (0..=nx).map(|i| sample(i, 0)).collect();
    for j in 0..ny {
        let next: Vec<f32> = (0..=nx).map(|i| sample(i, j + 1)).collect();
        for i in 0..nx {
            // esquinas: 0 sup-izq, 1 sup-der, 2 inf-der, 3 inf-izq
            let v = [prev[i], prev[i + 1], next[i + 1], next[i]];
            let (lo, hi) = v.iter().fold((f32::MAX, f32::MIN), |(l, h), &x| (l.min(x), h.max(x)));
            let k0 = (lo / p.spacing).floor() as i32 + 1;
            let k1 = (hi / p.spacing).floor() as i32;
            let (x0, y0) = (i as f32 * p.cell, j as f32 * p.cell);
            for k in k0..=k1 {
                cell_segments(&mut out, &v, k as f32 * p.spacing, x0, y0, p.cell);
            }
        }
        prev = next;
    }
    out
}

fn cell_segments(out: &mut Vec<Segment>, v: &[f32; 4], level: f32, x0: f32, y0: f32, s: f32) {
    let corner = [(x0, y0), (x0 + s, y0), (x0 + s, y0 + s), (x0, y0 + s)];
    // Punto de cruce en la arista e (entre esquina e y e+1).
    let edge = |e: usize| {
        let (a, b) = (e, (e + 1) % 4);
        let t = (level - v[a]) / (v[b] - v[a]);
        (corner[a].0 + (corner[b].0 - corner[a].0) * t, corner[a].1 + (corner[b].1 - corner[a].1) * t)
    };
    let mut case = 0u8;
    for (n, &val) in v.iter().enumerate() {
        if val >= level {
            case |= 1 << n;
        }
    }
    let mut seg = |a: usize, b: usize| out.push(Segment { a: edge(a), b: edge(b) });
    match case {
        0 | 15 => {}
        1 | 14 => seg(3, 0),
        2 | 13 => seg(0, 1),
        3 | 12 => seg(3, 1),
        4 | 11 => seg(1, 2),
        6 | 9 => seg(0, 2),
        7 | 8 => seg(2, 3),
        // Casos de silla: se desambigua con el valor medio.
        5 | 10 => {
            let centre_high = (v[0] + v[1] + v[2] + v[3]) / 4.0 >= level;
            if (case == 5) == centre_high {
                seg(3, 2);
                seg(0, 1);
            } else {
                seg(3, 0);
                seg(1, 2);
            }
        }
        _ => unreachable!(),
    }
}
