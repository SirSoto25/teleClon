//! Stickers en la UI: el pack propio de `tc-stickers` dibujado a handles de imagen (una vez por
//! sticker), la variante animada (movimiento procedural sobre el mismo dibujo), recientes con
//! `tc_stickers::Recents` y el estado del selector (pestañas, cursor de teclado).
//!
//! Los stickers de la red (`Sticker.file`) se descargan por `MediaApi` y se dibujan como imagen
//! (ver `sticker_img.rs`): WebP estático entero, TGS con su miniatura WebP (o el primer fotograma
//! con la feature `lottie`) y WebM con su miniatura. Mientras cargan, o sin archivo, el emoji hace de
//! respaldo. El pack propio de Parlo sigue como pack local adicional cuando la cuenta no lo tiene.

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

/// ¿Se anima en pantalla? Solo el pack propio (movimiento procedural); los TGS/WebM de la red se
/// dibujan estáticos por ahora (ver `sticker_img.rs`).
pub fn is_animated(s: &Sticker) -> bool {
    is_animated_format(s) && art_index(s).is_some()
}

/// ¿El formato es animado (TGS o WebM)?
pub fn is_animated_format(s: &Sticker) -> bool {
    matches!(s.format, StickerFormat::Lottie | StickerFormat::Webm)
}

/// Stickers de la cuenta que no son packs: recientes y favoritos del servidor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetLists {
    pub recent: Vec<Sticker>,
    pub faved: Vec<Sticker>,
    /// El pack propio se añadió en la UI porque la cuenta no lo tiene: sus stickers no existen en
    /// la red y se envían como archivo WebP.
    pub local_pack: bool,
}

/// Añade el pack propio al final si la cuenta no lo trae (backend real). Devuelve si lo añadió.
pub fn with_local_pack(sets: &mut Vec<StickerSet>) -> bool {
    if sets.iter().any(|s| s.id == pack::SET_ID) {
        return false;
    }
    sets.push(pack::sample_set());
    true
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
    /// Favoritos de la cuenta.
    Faved,
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

/// Pestañas visibles: «Recientes» (si hay), «Favoritos» (si hay) y un set por pestaña.
pub fn tabs(sets: &[StickerSet], recents: &Recents) -> Vec<Tab> {
    tabs_with(sets, recents, &NetLists::default())
}

/// Como `tabs`, contando también los recientes y favoritos del servidor.
pub fn tabs_with(sets: &[StickerSet], recents: &Recents, net: &NetLists) -> Vec<Tab> {
    let mut v = Vec::with_capacity(sets.len() + 2);
    if !recents.ids().is_empty() || !net.recent.is_empty() {
        v.push(Tab::Recent);
    }
    if !net.faved.is_empty() {
        v.push(Tab::Faved);
    }
    v.extend((0..sets.len()).map(Tab::Set));
    v
}

pub fn find(sets: &[StickerSet], id: StickerId) -> Option<&Sticker> {
    sets.iter().flat_map(|s| s.stickers.iter()).find(|s| s.id == id)
}

/// Busca en los packs y, si no está, en los recientes y favoritos del servidor.
pub fn find_any<'a>(sets: &'a [StickerSet], net: &'a NetLists, id: StickerId) -> Option<&'a Sticker> {
    find(sets, id).or_else(|| net.recent.iter().chain(&net.faved).find(|s| s.id == id))
}

/// Stickers de una pestaña, en orden.
pub fn tab_stickers<'a>(tab: Tab, sets: &'a [StickerSet], recents: &Recents) -> Vec<&'a Sticker> {
    match tab {
        Tab::Recent => recents.ids().iter().filter_map(|id| find(sets, *id)).collect(),
        Tab::Faved => Vec::new(),
        Tab::Set(i) => sets.get(i).map(|s| s.stickers.iter().collect()).unwrap_or_default(),
    }
}

/// Recientes de esta app primero, luego los del servidor (sin repetir; como mucho `RECENT_MAX`).
pub const RECENT_MAX: usize = 40;

/// Como `tab_stickers`, con recientes y favoritos del servidor.
pub fn tab_stickers_with<'a>(tab: Tab, sets: &'a [StickerSet], recents: &Recents, net: &'a NetLists) -> Vec<&'a Sticker> {
    match tab {
        Tab::Recent => {
            let mut seen = std::collections::HashSet::new();
            recents
                .ids()
                .iter()
                .filter_map(|id| find_any(sets, net, *id))
                .chain(net.recent.iter())
                .filter(|s| seen.insert(s.id))
                .take(RECENT_MAX)
                .collect()
        }
        Tab::Faved => net.faved.iter().collect(),
        Tab::Set(_) => tab_stickers(tab, sets, recents),
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

    fn net_sticker(id: i64) -> Sticker {
        Sticker { id: StickerId(id), set_id: StickerSetId(77), emoji: "😺".into(), width: 512, height: 512, format: StickerFormat::Lottie, file: None }
    }

    #[test]
    fn recientes_y_favoritos_del_servidor() {
        let mut sets = vec![StickerSet { id: StickerSetId(77), short_name: "red".into(), title: "Red".into(), stickers: vec![net_sticker(1), net_sticker(2)] }];
        assert!(with_local_pack(&mut sets), "la cuenta real no trae el pack propio");
        assert!(!with_local_pack(&mut sets), "no se duplica");
        assert_eq!(sets.len(), 2);
        let net = NetLists { recent: vec![net_sticker(2), net_sticker(9)], faved: vec![net_sticker(1)], local_pack: true };
        let mut r = Recents::new(30);
        r.record(pack::sticker_id(3));
        r.record(StickerId(2));
        assert_eq!(tabs_with(&sets, &r, &net), vec![Tab::Recent, Tab::Faved, Tab::Set(0), Tab::Set(1)]);
        let ids: Vec<_> = tab_stickers_with(Tab::Recent, &sets, &r, &net).iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![StickerId(2), pack::sticker_id(3), StickerId(9)], "locales primero, sin repetir");
        assert_eq!(tab_stickers_with(Tab::Faved, &sets, &r, &net).len(), 1);
        assert_eq!(find_any(&sets, &net, StickerId(9)).map(|s| s.id), Some(StickerId(9)));
        // Los TGS de la red no usan la animación procedural del pack propio.
        assert!(is_animated_format(&net_sticker(1)) && !is_animated(&net_sticker(1)));
        assert_eq!(tabs_with(&sets, &Recents::new(30), &NetLists::default()), vec![Tab::Set(0), Tab::Set(1)]);
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
