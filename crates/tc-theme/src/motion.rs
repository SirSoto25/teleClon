//! Muelles de movimiento, curvas de respaldo y preferencia de movimiento reducido.

/// Muelle amortiguado (masa 1 por defecto). Unidades: rigidez N/px, amortiguación N·s/px.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spring {
    pub stiffness: f32,
    pub damping: f32,
    pub mass: f32,
}

impl Spring {
    /// hover, press, toggles, checkbox.
    pub const RAPIDO: Spring = Spring { stiffness: 1840.0, damping: 77.0, mass: 1.0 };
    /// paneles, menús, entrada de burbuja, cambio de chat.
    pub const ESTANDAR: Spring = Spring { stiffness: 720.0, damping: 47.0, mass: 1.0 };
    /// diálogos, toasts, panel de info.
    pub const SUAVE: Spring = Spring { stiffness: 600.0, damping: 45.0, mass: 1.0 };
    /// solo botón enviar, reacción añadida, badge nuevo.
    pub const REBOTE: Spring = Spring { stiffness: 900.0, damping: 43.0, mass: 1.0 };

    /// Avanza `dt` segundos desde `state` hacia `target` (Euler semi-implícito con subpasos).
    /// Interrumpible: un muelle nuevo puede arrancar con la posición y velocidad actuales.
    pub fn step(self, state: SpringState, target: f32, dt: f32) -> SpringState {
        let n = (dt / 0.001).ceil().max(1.0) as u32;
        let h = dt / n as f32;
        let (mut x, mut v) = (state.position, state.velocity);
        for _ in 0..n {
            let a = (-self.stiffness * (x - target) - self.damping * v) / self.mass;
            v += a * h;
            x += v * h;
        }
        SpringState { position: x, velocity: v }
    }

    /// Milisegundos hasta quedar de forma definitiva dentro de `eps` (fracción del recorrido)
    /// del destino, para un recorrido 0 → 1. Máximo 5 s.
    pub fn settle_ms(self, eps: f32) -> f32 {
        let mut s = SpringState { position: 0.0, velocity: 0.0 };
        let mut last_out = 0.0_f32;
        for ms in 1..=5000 {
            s = self.step(s, 1.0, 0.001);
            if (s.position - 1.0).abs() > eps {
                last_out = ms as f32;
            }
        }
        last_out + 1.0
    }
}

/// Posición y velocidad de una propiedad animada.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SpringState {
    pub position: f32,
    pub velocity: f32,
}

/// Curva cúbica de Bézier de respaldo (sin física), con duración.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Easing {
    pub ms: u32,
    pub bezier: [f32; 4],
}

impl Easing {
    pub const RAPIDO: Easing = Easing { ms: 120, bezier: [0.2, 0.0, 0.0, 1.0] };
    pub const ESTANDAR: Easing = Easing { ms: 180, bezier: [0.2, 0.8, 0.2, 1.0] };
    pub const SUAVE: Easing = Easing { ms: 220, bezier: [0.3, 0.0, 0.0, 1.0] };
    pub const SALIDA: Easing = Easing { ms: 140, bezier: [0.4, 0.0, 1.0, 1.0] };
}

/// Excepciones de duración fija.
pub const HIGHLIGHT_FADE_MS: u32 = 900;
pub const TYPING_DOTS_PERIOD_MS: u32 = 1200;
pub const SKELETON_PULSE_PERIOD_MS: u32 = 1400;

/// Tope de las transiciones de opacidad con movimiento reducido.
pub const REDUCED_OPACITY_MAX_MS: u32 = 100;

/// Propiedad que se anima (solo se permiten estas tres; nunca layout ni tamaño de texto).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Property {
    Position,
    Scale,
    Opacity,
}

/// Preferencia de movimiento.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MotionPreference {
    #[default]
    Full,
    Reduced,
}

impl MotionPreference {
    pub fn from_reduced(reduced: bool) -> Self {
        if reduced { Self::Reduced } else { Self::Full }
    }

    pub fn is_reduced(self) -> bool {
        self == Self::Reduced
    }

    /// Duración efectiva: con movimiento reducido es 0, salvo opacidad, limitada a 100 ms.
    pub fn duration_ms(self, requested_ms: u32, property: Property) -> u32 {
        match (self, property) {
            (Self::Full, _) => requested_ms,
            (Self::Reduced, Property::Opacity) => requested_ms.min(REDUCED_OPACITY_MAX_MS),
            (Self::Reduced, _) => 0,
        }
    }

    /// ¿Debe usarse el muelle (true) o saltar directo al valor final (false)?
    pub fn animates(self, property: Property) -> bool {
        self.duration_ms(1, property) > 0
    }
}
