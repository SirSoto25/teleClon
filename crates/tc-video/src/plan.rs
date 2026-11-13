//! Cuántos fotogramas guardar y a qué tamaño para que una animación quepa en su presupuesto de
//! memoria sin perder el bucle entero: primero se quitan los fotogramas que el reloj de la UI no
//! llegaría a enseñar, luego se reduce el tamaño (hasta `min_side`) y, solo si aún no cabe, se
//! quitan más fotogramas. Los que se quitan se reparten por todo el clip y su tiempo pasa al
//! fotograma anterior, así la duración del bucle no cambia.

/// Topes de una animación decodificada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Lado mayor como mucho (px).
    pub max_side: u32,
    /// Lado mayor por debajo del cual se prefiere quitar fotogramas a encoger más.
    pub min_side: u32,
    /// Fotogramas como mucho.
    pub max_frames: usize,
    /// Bytes RGBA como mucho de todos los fotogramas.
    pub max_bytes: usize,
    /// Duración mínima útil de un fotograma (el periodo del reloj de la UI): los más cortos se agrupan.
    pub min_frame_ms: u32,
}

/// Resultado: guardar `frames` fotogramas (repartidos con [`pick`]) a `width`×`height`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plan {
    pub frames: usize,
    pub width: u32,
    pub height: u32,
}

impl Plan {
    /// Lado mayor del plan.
    pub fn side(&self) -> u32 {
        self.width.max(self.height)
    }
}

/// Encaja `w`×`h` en un lado mayor de `side` (sin agrandar).
pub fn fit(w: u32, h: u32, side: u32) -> (u32, u32) {
    let (w, h) = (w.max(1), h.max(1));
    let side = side.max(1);
    if w.max(h) <= side {
        (w, h)
    } else if w >= h {
        (side, ((u64::from(h) * u64::from(side) / u64::from(w)) as u32).max(1))
    } else {
        (((u64::from(w) * u64::from(side) / u64::from(h)) as u32).max(1), side)
    }
}

fn frame_bytes((w, h): (u32, u32)) -> usize {
    w as usize * h as usize * 4
}

/// Plan para un clip de `src_w`×`src_h` con esas duraciones (ms, en orden de presentación).
pub fn plan(src_w: u32, src_h: u32, durations_ms: &[u32], l: &Limits) -> Plan {
    let n = durations_ms.len().max(1);
    let total: u64 = durations_ms.iter().map(|d| u64::from(*d)).sum();
    let by_clock = if l.min_frame_ms > 0 && total > 0 { total.div_ceil(u64::from(l.min_frame_ms)) as usize } else { n };
    let mut frames = n.min(by_clock).min(l.max_frames.max(1)).max(1);

    let side0 = l.max_side.min(src_w.max(src_h)).max(1);
    let mut size = fit(src_w, src_h, side0);
    let budget = l.max_bytes.max(1);
    if frames * frame_bytes(size) > budget {
        let s = (budget as f64 / (frames * frame_bytes(size)) as f64).sqrt();
        let side = ((f64::from(side0) * s).floor() as u32).max(l.min_side.min(side0)).max(1);
        size = fit(src_w, src_h, side);
    }
    frames = frames.min((budget / frame_bytes(size)).max(1));
    Plan { frames, width: size.0, height: size.1 }
}

/// Índices (crecientes, sin repetir) de los `keep` fotogramas que se guardan de `n`.
pub fn pick(n: usize, keep: usize) -> Vec<usize> {
    let keep = keep.min(n);
    (0..keep).map(|j| j * n / keep).collect()
}

/// Duración de cada fotograma guardado: la suya más la de los quitados que le siguen.
pub fn group_ms(durations_ms: &[u32], picks: &[usize]) -> Vec<u32> {
    picks
        .iter()
        .enumerate()
        .map(|(j, &a)| {
            let z = picks.get(j + 1).copied().unwrap_or(durations_ms.len());
            durations_ms.get(a..z).unwrap_or_default().iter().fold(0u32, |s, d| s.saturating_add(*d)).max(1)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const L: Limits = Limits { max_side: 320, min_side: 160, max_frames: 150, max_bytes: 10 << 20, min_frame_ms: 33 };

    #[test]
    fn un_clip_pequeno_se_queda_igual() {
        assert_eq!(plan(320, 180, &[40; 40], &L), Plan { frames: 40, width: 320, height: 180 });
        assert_eq!(plan(100, 60, &[33; 30], &L), Plan { frames: 30, width: 100, height: 60 }, "no agranda");
        assert_eq!(plan(640, 480, &[40; 10], &L).side(), 320);
    }

    #[test]
    fn quita_fotogramas_que_el_reloj_no_ensenaria() {
        // 60 fps durante 1 s → ~30 fotogramas; a 30 fps se quedan todos.
        assert_eq!(plan(160, 90, &[17; 60], &L).frames, 31);
        assert_eq!(plan(160, 90, &[33; 60], &L).frames, 60);
        // Más de `max_frames`: se quedan 150, repartidos por todo el clip.
        assert_eq!(plan(64, 64, &[40; 400], &L).frames, 150);
    }

    #[test]
    fn encoge_y_despues_quita_fotogramas_para_caber() {
        // 150 fotogramas de 320×320 (60 MiB) no caben en 10 MiB: encoge hasta 160 y quita los justos.
        let p = plan(480, 480, &[40; 150], &L);
        assert_eq!((p.width, p.height), (160, 160));
        assert_eq!(p.frames, L.max_bytes / (160 * 160 * 4));
        // Si basta con encoger, no se quitan fotogramas.
        let p = plan(320, 180, &[40; 100], &L);
        assert_eq!(p.frames, 100);
        assert!(100 * p.width as usize * p.height as usize * 4 <= L.max_bytes && p.width > 160, "{p:?}");
    }

    #[test]
    fn reparte_y_agrupa_duraciones() {
        assert_eq!(pick(5, 5), [0, 1, 2, 3, 4]);
        assert_eq!(pick(10, 4), [0, 2, 5, 7]);
        assert_eq!(pick(3, 9), [0, 1, 2]);
        assert_eq!(group_ms(&[10, 20, 30, 40, 50], &[0, 2, 3]), [30, 30, 90]);
        assert_eq!(group_ms(&[0, 0], &[0, 1]), [1, 1]);
        let d = [17u32; 60];
        assert_eq!(group_ms(&d, &pick(60, 31)).iter().sum::<u32>(), 17 * 60, "el bucle dura lo mismo");
        assert_eq!(fit(1000, 10, 320), (320, 3));
        assert_eq!(fit(10, 1000, 320), (3, 320));
    }
}
