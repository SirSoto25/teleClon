//! Motor de animación con muelles de `tc-theme` (interrumpibles) y movimiento reducido.
//!
//! Solo se animan posición, escala y opacidad. Con movimiento reducido, posición y escala saltan
//! al valor final y la opacidad usa una rampa lineal de como mucho 100 ms. La app solo se suscribe
//! a los frames de ventana mientras haya animaciones activas (CPU en reposo = 0).

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use tc_theme::motion::{Property, SpringState};
use tc_theme::{MotionPreference, Spring};

#[derive(Clone, Copy, Debug)]
enum Kind {
    Spring { spring: Spring, state: SpringState },
    Linear { from: f32, start: Instant, ms: u32 },
}

/// Curva de una animación: muelle, propiedad animada y duración de referencia (para la rampa
/// lineal de movimiento reducido).
#[derive(Clone, Copy, Debug)]
pub struct Curve {
    pub spring: Spring,
    pub prop: Property,
    pub ms: u32,
}

impl Curve {
    pub const fn new(spring: Spring, prop: Property, ms: u32) -> Self {
        Self { spring, prop, ms }
    }
}

#[derive(Clone, Copy, Debug)]
struct Anim {
    kind: Kind,
    target: f32,
}

/// Conjunto de animaciones identificadas por clave.
#[derive(Debug)]
pub struct Animator<K> {
    anims: HashMap<K, Anim>,
    last: Option<Instant>,
    pub pref: MotionPreference,
}

impl<K: Eq + Hash + Copy> Animator<K> {
    pub fn new(pref: MotionPreference) -> Self {
        Self { anims: HashMap::new(), last: None, pref }
    }

    pub fn is_active(&self) -> bool {
        !self.anims.is_empty()
    }

    /// Arranca (o reorienta) una animación de `from` a `to`. Si ya existe, hereda posición y
    /// velocidad (interrumpible). `ms` es la duración de referencia para la rampa reducida.
    pub fn start(&mut self, key: K, from: f32, to: f32, curve: Curve, now: Instant) {
        let Curve { spring, prop, ms } = curve;
        if !self.pref.animates(prop) {
            self.anims.remove(&key);
            return;
        }
        if self.anims.is_empty() {
            self.last = Some(now);
        }
        let kind = if self.pref.is_reduced() {
            let from = self.value(key, from);
            Kind::Linear { from, start: now, ms: self.pref.duration_ms(ms, prop).max(1) }
        } else {
            let state = match self.anims.get(&key).map(|a| a.kind) {
                Some(Kind::Spring { state, .. }) => state,
                _ => SpringState { position: from, velocity: 0.0 },
            };
            Kind::Spring { spring, state }
        };
        self.anims.insert(key, Anim { kind, target: to });
    }

    /// Valor actual, o `default` si la clave no se está animando.
    pub fn value(&self, key: K, default: f32) -> f32 {
        match self.anims.get(&key) {
            None => default,
            Some(a) => match a.kind {
                Kind::Spring { state, .. } => state.position,
                Kind::Linear { from, start, ms } => {
                    let t = (self.last.unwrap_or(start).saturating_duration_since(start).as_secs_f32() * 1000.0
                        / ms as f32)
                        .clamp(0.0, 1.0);
                    from + (a.target - from) * t
                }
            },
        }
    }

    /// Avanza todas las animaciones hasta `now` y retira las que han terminado.
    pub fn tick(&mut self, now: Instant) {
        let dt = self.last.map_or(Duration::ZERO, |l| now.saturating_duration_since(l)).min(Duration::from_millis(64));
        self.last = Some(now);
        let dt = dt.as_secs_f32();
        self.anims.retain(|_, a| match &mut a.kind {
            Kind::Spring { spring, state } => {
                *state = spring.step(*state, a.target, dt);
                let done = (state.position - a.target).abs() < 0.002 && state.velocity.abs() < 0.02;
                !done
            }
            Kind::Linear { start, ms, .. } => {
                now.saturating_duration_since(*start) < Duration::from_millis(u64::from(*ms))
            }
        });
        if self.anims.is_empty() {
            self.last = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn muelle_converge_y_termina() {
        let mut a: Animator<u8> = Animator::new(MotionPreference::Full);
        let t0 = Instant::now();
        a.start(1, 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), t0);
        assert!(a.is_active());
        let mut t = t0;
        for _ in 0..60 {
            t += Duration::from_millis(16);
            a.tick(t);
        }
        assert!(!a.is_active(), "debe asentarse en < 1 s");
        assert_eq!(a.value(1, 1.0), 1.0);
    }

    #[test]
    fn movimiento_reducido_salta_posicion_y_limita_opacidad() {
        let mut a: Animator<u8> = Animator::new(MotionPreference::Reduced);
        let t0 = Instant::now();
        a.start(1, 8.0, 0.0, Curve::new(Spring::ESTANDAR, Property::Position, 180), t0);
        assert!(!a.is_active(), "la posición no se anima");
        a.start(2, 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), t0);
        a.tick(t0 + Duration::from_millis(50));
        let mid = a.value(2, 1.0);
        assert!(mid > 0.3 && mid < 0.7, "rampa lineal de 100 ms: {mid}");
        a.tick(t0 + Duration::from_millis(101));
        assert!(!a.is_active());
    }

    #[test]
    fn interrumpible_hereda_posicion() {
        let mut a: Animator<u8> = Animator::new(MotionPreference::Full);
        let t0 = Instant::now();
        a.start(1, 0.0, 1.0, Curve::new(Spring::RAPIDO, Property::Scale, 120), t0);
        a.tick(t0 + Duration::from_millis(16));
        a.tick(t0 + Duration::from_millis(32));
        let p = a.value(1, 0.0);
        a.start(1, 0.0, 0.0, Curve::new(Spring::RAPIDO, Property::Scale, 120), t0 + Duration::from_millis(32));
        assert_eq!(a.value(1, -1.0), p, "no reinicia desde 0");
    }
}
