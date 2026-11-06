//! Generador congruencial lineal propio (sin dependencia `rand`): solo para datos de muestra.

#[derive(Debug, Clone)]
pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }
    /// Entero en `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        self.next_u32() as usize % n.max(1)
    }
    /// Verdadero con probabilidad `percent` %.
    pub fn chance(&mut self, percent: u32) -> bool {
        self.next_u32() % 100 < percent
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic() {
        let (mut a, mut b) = (Lcg::new(7), Lcg::new(7));
        for _ in 0..50 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
        assert_ne!(Lcg::new(1).next_u32(), Lcg::new(2).next_u32());
    }

    #[test]
    fn below_stays_in_range() {
        let mut r = Lcg::new(3);
        assert!((0..1000).all(|_| r.below(5) < 5));
    }
}
