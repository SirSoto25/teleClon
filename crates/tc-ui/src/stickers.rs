//! Stickers en la UI: el pack propio de `tc-stickers` dibujado a handles de imagen (una vez por
//! sticker), la variante animada (movimiento procedural sobre el mismo dibujo), recientes con
//! `tc_stickers::Recents` y el estado del selector (pestañas, cursor de teclado).
//!
//! Los stickers que llegan de la red (sin bytes en el modelo de `tc-core`) se muestran con su
//! emoji como respaldo.

use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::TAU;

use iced::widget::image;
use tc_core::{Sticker, StickerFormat, StickerId, StickerSet, StickerSetId};
use tc_stickers::{Recents, pack};

/// Set de la variante animada del pack propio. Mismo convenio que `tc_mock::ANIMATED_SET_ID`:
/// sticker `i` = `ANIMATED_SET_ID × 100 + i`, con el dibujo `i` del pack estático.
pub const ANIMATED_SET_ID: StickerSetId = StickerSetId(9_000_002);
/// Lado (px físicos) al que se dibuja cada sticker; la GPU lo reduce al tamaño de la celda.
pub const ART_PX: u32 = 256;
/// Duración de un ciclo de la animación procedural.
pub const LOOP_MS: f32 = 1600.0;
/// Celda del selector (§8: rejilla de 72 px) y lado del dibujo dentro de ella.
pub const CELL: f32 = 72.0;
pub const CELL_ART: f32 = 60.0;
pub const CELL_GAP: f32 = 4.0;
/// Alto del panel del selector.
pub const PANEL_H: f32 = 288.0;

/// Índice del dibujo del pack propio que corresponde a un sticker, si es nuestro.
pub fn art_index(s: &Sticker) -> Option<usize> {
    if s.set_id == pack::SET_ID {
        return pack::index_of(s.id);
    }
    if s.set_id == ANIMATED_SET_ID {
        let i = s.id.0.checked_sub(ANIMATED_SET_ID.0 * 100)?;
        return usize::try_from(i).ok().filter(|i| *i < pack::count());
    }
    None
}

pub fn is_animated(s: &Sticker) -> bool {
    matches!(s.format, StickerFormat::Lottie | StickerFormat::Webm)
}

/// Transformación del instante `phase` (0..1) del ciclo del sticker animado `index`:
/// `(escala, giro en radianes)`. La fase 0 es la identidad (= primer fotograma, el que se ve
/// con movimiento reducido).
pub fn motion(index: usize, phase: f32) -> (f32, f32) {
    let t = TAU * phase.rem_euclid(1.0);
    let s = t.sin();
    match index % 4 {
        // Bote suave.
        0 => (1.0 + 0.08 * s * s, 0.0),
        // Balanceo.
        1 => (1.0, 0.16 * s),
        // Latido con un leve giro.
        2 => (1.0 + 0.05 * s, 0.06 * (2.0 * t).sin()),
        // Encoger y girar.
        _ => (1.0 - 0.06 * s * s, -0.12 * s),
    }
}

/// Handles del pack dibujados bajo demanda (el dibujo por campos de distancia cuesta unos ms;
/// se hace una vez por sticker y se reutiliza: rehacer el handle obligaría a resubir la textura).
#[derive(Default)]
pub struct Art {
    handles: RefCell<HashMap<usize, Option<image::Handle>>>,
}

impl std::fmt::Debug for Art {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Art").field("cached", &self.handles.borrow().len()).finish()
    }
}

impl Art {
    pub fn handle(&self, s: &Sticker) -> Option<image::Handle> {
        let idx = art_index(s)?;
        self.handles
            .borrow_mut()
            .entry(idx)
            .or_insert_with(|| pack::render(idx, ART_PX).map(|f| image::Handle::from_rgba(f.width, f.height, f.rgba)))
            .clone()
    }

    pub fn len(&self) -> usize {
        self.handles.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Pestaña del selector.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Recent,
    Set(usize),
}

/// Movimiento del cursor del selector con el teclado.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    Left,
    Right,
    Up,
    Down,
    NextTab,
    PrevTab,
    /// Enviar el sticker bajo el cursor.
    Activate,
}

/// Estado del selector de stickers (panel junto al compositor).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Picker {
    pub open: bool,
    /// Índice en `tabs(..)`.
    pub tab: usize,
    /// Sticker resaltado para el teclado (índice en la pestaña actual).
    pub cursor: Option<usize>,
}

/// Pestañas visibles: «Recientes» (si hay) y un set por pestaña.
pub fn tabs(sets: &[StickerSet], recents: &Recents) -> Vec<Tab> {
    let mut v = Vec::with_capacity(sets.len() + 1);
    if !recents.ids().is_empty() {
        v.push(Tab::Recent);
    }
    v.extend((0..sets.len()).map(Tab::Set));
    v
}

pub fn find(sets: &[StickerSet], id: StickerId) -> Option<&Sticker> {
    sets.iter().flat_map(|s| s.stickers.iter()).find(|s| s.id == id)
}

/// Stickers de una pestaña, en orden.
pub fn tab_stickers<'a>(tab: Tab, sets: &'a [StickerSet], recents: &Recents) -> Vec<&'a Sticker> {
    match tab {
        Tab::Recent => recents.ids().iter().filter_map(|id| find(sets, *id)).collect(),
        Tab::Set(i) => sets.get(i).map(|s| s.stickers.iter().collect()).unwrap_or_default(),
    }
}

impl Picker {
    /// Abre el selector en «Recientes» si hay, o en el primer set.
    pub fn open(&mut self) {
        self.open = true;
        self.tab = 0;
        self.cursor = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.cursor = None;
    }

    pub fn select_tab(&mut self, tab: usize, n_tabs: usize) {
        self.tab = tab.min(n_tabs.saturating_sub(1));
        self.cursor = None;
    }

    /// Aplica una tecla de navegación. `len` = stickers de la pestaña, `cols` = columnas de la
    /// rejilla. Devuelve el índice a enviar si la tecla es `Activate` con cursor.
    pub fn nav(&mut self, nav: Nav, len: usize, cols: usize, n_tabs: usize) -> Option<usize> {
        let cols = cols.max(1);
        match nav {
            Nav::NextTab | Nav::PrevTab => {
                if n_tabs > 0 {
                    let next = if nav == Nav::NextTab { (self.tab + 1) % n_tabs } else { (self.tab + n_tabs - 1) % n_tabs };
                    self.select_tab(next, n_tabs);
                }
                None
            }
            Nav::Activate => self.cursor.filter(|c| *c < len),
            _ if len == 0 => None,
            _ => {
                let cur = match self.cursor {
                    None => 0,
                    Some(c) => {
                        let c = c.min(len - 1);
                        match nav {
                            Nav::Left => c.saturating_sub(1),
                            Nav::Right => (c + 1).min(len - 1),
                            Nav::Up => c.checked_sub(cols).unwrap_or(c),
                            Nav::Down if c + cols < len => c + cols,
                            _ => c,
                        }
                    }
                };
                self.cursor = Some(cur);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_pack_y_su_variante_animada_tienen_dibujo() {
        let set = pack::sample_set();
        for (i, s) in set.stickers.iter().enumerate() {
            assert_eq!(art_index(s), Some(i));
            assert!(!is_animated(s));
            let anim = Sticker { id: StickerId(ANIMATED_SET_ID.0 * 100 + i as i64), set_id: ANIMATED_SET_ID, format: StickerFormat::Lottie, ..s.clone() };
            assert_eq!(art_index(&anim), Some(i));
            assert!(is_animated(&anim));
        }
        let ajeno = Sticker { id: StickerId(5), set_id: StickerSetId(77), ..set.stickers[0].clone() };
        assert_eq!(art_index(&ajeno), None);
        let fuera = Sticker { id: StickerId(ANIMATED_SET_ID.0 * 100 + 99), set_id: ANIMATED_SET_ID, ..set.stickers[0].clone() };
        assert_eq!(art_index(&fuera), None);
    }

    #[test]
    fn el_primer_fotograma_es_la_identidad_y_el_ciclo_se_cierra() {
        for i in 0..pack::count() {
            assert_eq!(motion(i, 0.0), (1.0, 0.0));
            let (a, b) = (motion(i, 0.999_99), motion(i, 0.0));
            assert!((a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3);
            let (s, r) = motion(i, 0.3);
            assert!((0.85..=1.15).contains(&s) && r.abs() < 0.3);
        }
    }

    #[test]
    fn la_rejilla_se_recorre_con_flechas_sin_salirse() {
        let mut p = Picker::default();
        p.open();
        // 12 stickers en 5 columnas.
        assert_eq!(p.nav(Nav::Right, 12, 5, 2), None);
        assert_eq!(p.cursor, Some(0)); // la primera flecha solo coloca el cursor
        p.nav(Nav::Left, 12, 5, 2);
        assert_eq!(p.cursor, Some(0));
        p.nav(Nav::Down, 12, 5, 2);
        p.nav(Nav::Down, 12, 5, 2);
        assert_eq!(p.cursor, Some(10));
        p.nav(Nav::Down, 12, 5, 2); // no hay fila debajo
        assert_eq!(p.cursor, Some(10));
        p.nav(Nav::Right, 12, 5, 2);
        p.nav(Nav::Right, 12, 5, 2);
        assert_eq!(p.cursor, Some(11));
        p.nav(Nav::Up, 12, 5, 2);
        assert_eq!(p.cursor, Some(6));
        assert_eq!(p.nav(Nav::Activate, 12, 5, 2), Some(6));
        p.nav(Nav::NextTab, 12, 5, 2);
        assert_eq!((p.tab, p.cursor), (1, None));
        p.nav(Nav::NextTab, 12, 5, 2);
        assert_eq!(p.tab, 0);
        p.nav(Nav::PrevTab, 12, 5, 2);
        assert_eq!(p.tab, 1);
        assert_eq!(p.nav(Nav::Activate, 12, 5, 2), None);
        assert_eq!(p.nav(Nav::Down, 0, 5, 2), None);
    }

    #[test]
    fn recientes_primero_y_solo_si_hay() {
        let sets = vec![pack::sample_set()];
        let mut r = Recents::new(30);
        assert_eq!(tabs(&sets, &r), vec![Tab::Set(0)]);
        r.record(pack::sticker_id(4));
        r.record(pack::sticker_id(2));
        assert_eq!(tabs(&sets, &r), vec![Tab::Recent, Tab::Set(0)]);
        let ids: Vec<_> = tab_stickers(Tab::Recent, &sets, &r).iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![pack::sticker_id(2), pack::sticker_id(4)]);
        assert_eq!(tab_stickers(Tab::Set(0), &sets, &r).len(), pack::count());
    }

    #[test]
    fn el_dibujo_se_hace_una_vez_por_sticker() {
        let art = Art::default();
        let s = &pack::sample_set().stickers[1];
        let a = art.handle(s).expect("dibujo");
        let b = art.handle(s).expect("dibujo");
        assert_eq!(a.id(), b.id());
        assert_eq!(art.len(), 1);
    }
}
