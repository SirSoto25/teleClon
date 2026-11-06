//! Pack propio ORIGINAL «Brasa» (12 stickers) dibujado por código con formas vectoriales
//! (campos de distancia con suavizado) y la paleta oscura de `tc-theme`. Sin assets de terceros.

use std::f32::consts::{PI, TAU};

use tc_core::{Sticker, StickerFormat, StickerId, StickerSet, StickerSetId};
use tc_theme::{Rgb, palette::DARK};

use crate::Frame;

pub const SET_ID: StickerSetId = StickerSetId(9_000_001);
pub const SET_SHORT_NAME: &str = "brasa_basicos";
pub const SET_TITLE: &str = "Brasa básicos";
/// Lado nativo de los stickers del pack.
pub const NATIVE_SIZE: u32 = 512;

type P = [f32; 2];

enum Shape {
    Circle(P, f32),
    Round { c: P, half: P, r: f32 },
    Seg(P, P, f32),
    Poly(Vec<P>),
    Sub(Box<Shape>, Box<Shape>),
}

impl Shape {
    /// Distancia con signo (negativa dentro), en coordenadas unitarias.
    fn sdf(&self, p: P) -> f32 {
        match self {
            Self::Circle(c, r) => len(sub(p, *c)) - r,
            Self::Round { c, half, r } => {
                let q = [(p[0] - c[0]).abs() - half[0] + r, (p[1] - c[1]).abs() - half[1] + r];
                len([q[0].max(0.0), q[1].max(0.0)]) + q[0].max(q[1]).min(0.0) - r
            }
            Self::Seg(a, b, r) => seg_dist(p, *a, *b) - r,
            Self::Poly(v) => poly_sdf(v, p),
            Self::Sub(a, b) => a.sdf(p).max(-b.sdf(p)),
        }
    }
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn len(a: P) -> f32 {
    a[0].hypot(a[1])
}
fn seg_dist(p: P, a: P, b: P) -> f32 {
    let pa = sub(p, a);
    let ba = sub(b, a);
    let d = ba[0] * ba[0] + ba[1] * ba[1];
    let h = if d == 0.0 { 0.0 } else { ((pa[0] * ba[0] + pa[1] * ba[1]) / d).clamp(0.0, 1.0) };
    len([pa[0] - ba[0] * h, pa[1] - ba[1] * h])
}
fn poly_sdf(v: &[P], p: P) -> f32 {
    let mut d = f32::MAX;
    let mut inside = false;
    for i in 0..v.len() {
        let a = v[i];
        let b = v[(i + 1) % v.len()];
        d = d.min(seg_dist(p, a, b));
        if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    if inside { -d } else { d }
}

struct Layer(Shape, Rgb);

fn star(c: P, outer: f32, inner: f32, n: usize) -> Vec<P> {
    (0..n * 2)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = -PI / 2.0 + i as f32 * PI / n as f32;
            [c[0] + r * a.cos(), c[1] + r * a.sin()]
        })
        .collect()
}

fn dots(c: P, r_in: f32, r_out: f32, w: f32, n: usize) -> Vec<Layer> {
    (0..n)
        .map(|i| {
            let a = i as f32 * TAU / n as f32;
            Layer(
                Shape::Seg(
                    [c[0] + r_in * a.cos(), c[1] + r_in * a.sin()],
                    [c[0] + r_out * a.cos(), c[1] + r_out * a.sin()],
                    w,
                ),
                DARK.warning,
            )
        })
        .collect()
}

/// Definición de un sticker: emoji asociado, nombre y capas (de atrás hacia delante).
struct Def {
    emoji: &'static str,
    name: &'static str,
    layers: fn() -> Vec<Layer>,
}

const WHITE: Rgb = Rgb { r: 255, g: 255, b: 255 };
const INK: Rgb = Rgb { r: 28, g: 22, b: 20 };

const DEFS: [Def; 12] = [
    Def {
        emoji: "🔥",
        name: "brasa",
        layers: || {
            vec![
                Layer(Shape::Circle([0.5, 0.60], 0.26), DARK.accent),
                Layer(Shape::Poly(vec![[0.26, 0.52], [0.74, 0.52], [0.52, 0.10]]), DARK.accent),
                Layer(Shape::Circle([0.5, 0.64], 0.14), DARK.warning),
                Layer(Shape::Poly(vec![[0.38, 0.62], [0.62, 0.62], [0.50, 0.38]]), DARK.warning),
            ]
        },
    },
    Def {
        emoji: "❤",
        name: "corazon",
        layers: || {
            vec![
                Layer(Shape::Circle([0.355, 0.40], 0.18), DARK.danger_solid),
                Layer(Shape::Circle([0.645, 0.40], 0.18), DARK.danger_solid),
                Layer(Shape::Poly(vec![[0.19, 0.47], [0.81, 0.47], [0.50, 0.82]]), DARK.danger_solid),
                Layer(Shape::Circle([0.31, 0.34], 0.04), WHITE),
            ]
        },
    },
    Def {
        emoji: "⭐",
        name: "estrella",
        layers: || {
            vec![
                Layer(Shape::Poly(star([0.5, 0.54], 0.40, 0.17, 5)), DARK.warning),
                Layer(Shape::Poly(star([0.5, 0.54], 0.22, 0.09, 5)), DARK.accent),
            ]
        },
    },
    Def {
        emoji: "🙂",
        name: "sonrisa",
        layers: || {
            let mut v = vec![
                Layer(Shape::Circle([0.5, 0.5], 0.40), DARK.warning),
                Layer(Shape::Circle([0.37, 0.42], 0.05), INK),
                Layer(Shape::Circle([0.63, 0.42], 0.05), INK),
            ];
            let pts: Vec<P> = (0..=8)
                .map(|i| {
                    let a = 0.15 * PI + i as f32 * 0.70 * PI / 8.0;
                    [0.5 + 0.20 * a.cos(), 0.52 + 0.18 * a.sin()]
                })
                .collect();
            for w in pts.windows(2) {
                v.push(Layer(Shape::Seg(w[0], w[1], 0.025), INK));
            }
            v
        },
    },
    Def {
        emoji: "🌙",
        name: "luna",
        layers: || {
            vec![Layer(
                Shape::Sub(
                    Box::new(Shape::Circle([0.48, 0.5], 0.36)),
                    Box::new(Shape::Circle([0.64, 0.42], 0.31)),
                ),
                DARK.name_3,
            )]
        },
    },
    Def {
        emoji: "💧",
        name: "gota",
        layers: || {
            vec![
                Layer(Shape::Circle([0.5, 0.62], 0.25), DARK.name_5),
                Layer(Shape::Poly(vec![[0.27, 0.56], [0.73, 0.56], [0.5, 0.12]]), DARK.name_5),
                Layer(Shape::Circle([0.42, 0.62], 0.05), WHITE),
            ]
        },
    },
    Def {
        emoji: "✅",
        name: "visto",
        layers: || {
            vec![
                Layer(Shape::Circle([0.5, 0.5], 0.40), DARK.success),
                Layer(Shape::Seg([0.30, 0.52], [0.44, 0.66], 0.045), WHITE),
                Layer(Shape::Seg([0.44, 0.66], [0.71, 0.36], 0.045), WHITE),
            ]
        },
    },
    Def {
        emoji: "⚡",
        name: "rayo",
        layers: || {
            vec![Layer(
                Shape::Poly(vec![
                    [0.58, 0.08],
                    [0.28, 0.54],
                    [0.47, 0.54],
                    [0.40, 0.92],
                    [0.72, 0.43],
                    [0.53, 0.43],
                ]),
                DARK.warning,
            )]
        },
    },
    Def {
        emoji: "✉",
        name: "sobre",
        layers: || {
            vec![
                Layer(Shape::Round { c: [0.5, 0.5], half: [0.38, 0.27], r: 0.05 }, DARK.name_7),
                Layer(Shape::Seg([0.15, 0.31], [0.5, 0.56], 0.02), WHITE),
                Layer(Shape::Seg([0.85, 0.31], [0.5, 0.56], 0.02), WHITE),
            ]
        },
    },
    Def {
        emoji: "🎈",
        name: "globo",
        layers: || {
            vec![
                Layer(Shape::Seg([0.5, 0.66], [0.56, 0.92], 0.008), DARK.text_muted),
                Layer(Shape::Poly(vec![[0.45, 0.70], [0.55, 0.70], [0.5, 0.62]]), DARK.name_4),
                Layer(Shape::Circle([0.5, 0.40], 0.28), DARK.name_4),
                Layer(Shape::Circle([0.40, 0.30], 0.05), WHITE),
            ]
        },
    },
    Def {
        emoji: "☀",
        name: "sol",
        layers: || {
            let mut v = dots([0.5, 0.5], 0.30, 0.43, 0.025, 8);
            v.push(Layer(Shape::Circle([0.5, 0.5], 0.23), DARK.warning));
            v.push(Layer(Shape::Circle([0.5, 0.5], 0.13), DARK.accent));
            v
        },
    },
    Def {
        emoji: "💬",
        name: "mensaje",
        layers: || {
            vec![
                Layer(Shape::Round { c: [0.5, 0.45], half: [0.38, 0.28], r: 0.14 }, DARK.accent),
                Layer(Shape::Poly(vec![[0.28, 0.65], [0.48, 0.65], [0.25, 0.88]]), DARK.accent),
                Layer(Shape::Circle([0.35, 0.45], 0.04), WHITE),
                Layer(Shape::Circle([0.5, 0.45], 0.04), WHITE),
                Layer(Shape::Circle([0.65, 0.45], 0.04), WHITE),
            ]
        },
    },
];

/// Id estable del sticker `index` (0..12) del pack.
pub fn sticker_id(index: usize) -> StickerId {
    StickerId(SET_ID.0 * 100 + index as i64)
}

/// Pack como modelo de dominio de `tc-core`.
pub fn sample_set() -> StickerSet {
    StickerSet {
        id: SET_ID,
        short_name: SET_SHORT_NAME.to_owned(),
        title: SET_TITLE.to_owned(),
        stickers: DEFS
            .iter()
            .enumerate()
            .map(|(i, d)| Sticker {
                id: sticker_id(i),
                set_id: SET_ID,
                emoji: d.emoji.to_owned(),
                width: NATIVE_SIZE,
                height: NATIVE_SIZE,
                format: StickerFormat::Webp,
            })
            .collect(),
    }
}

/// Nombre corto (sin acentos) del sticker `index`.
pub fn sticker_name(index: usize) -> Option<&'static str> {
    DEFS.get(index).map(|d| d.name)
}

/// Índice dentro del pack de un id, si pertenece al pack.
pub fn index_of(id: StickerId) -> Option<usize> {
    let i = id.0.checked_sub(SET_ID.0 * 100)?;
    usize::try_from(i).ok().filter(|i| *i < DEFS.len())
}

/// Dibuja el sticker `index` a `size`×`size` RGBA (sin premultiplicar). `None` si el índice no existe
/// o `size` es 0.
pub fn render(index: usize, size: u32) -> Option<Frame> {
    let def = DEFS.get(index)?;
    if size == 0 {
        return None;
    }
    let layers = (def.layers)();
    let n = size as usize;
    let px = 1.0 / size as f32;
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let p = [(x as f32 + 0.5) * px, (y as f32 + 0.5) * px];
            let (mut c, mut a) = ([0f32; 3], 0f32);
            for Layer(shape, col) in &layers {
                let cov = (0.5 - shape.sdf(p) / px).clamp(0.0, 1.0);
                if cov <= 0.0 {
                    continue;
                }
                let out_a = cov + a * (1.0 - cov);
                let src = [f32::from(col.r), f32::from(col.g), f32::from(col.b)];
                for k in 0..3 {
                    c[k] = (src[k] * cov + c[k] * a * (1.0 - cov)) / out_a;
                }
                a = out_a;
            }
            let i = (y * n + x) * 4;
            rgba[i] = c[0].round() as u8;
            rgba[i + 1] = c[1].round() as u8;
            rgba[i + 2] = c[2].round() as u8;
            rgba[i + 3] = (a * 255.0).round() as u8;
        }
    }
    Some(Frame { width: size, height: size, rgba })
}

/// Dibuja un sticker del pack por id.
pub fn render_id(id: StickerId, size: u32) -> Option<Frame> {
    render(index_of(id)?, size)
}

/// Número de stickers del pack.
pub const fn count() -> usize {
    DEFS.len()
}
