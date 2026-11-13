//! Lectura de una conversación como en Telegram:
//!
//! - Con mensajes sin leer, la primera página es la ventana alrededor del último leído
//!   (`Anchor::Around(read_inbox_max_id)`: contexto leído encima y los primeros no leídos debajo) y
//!   la vista se coloca en la marca «Sin leer desde aquí», no al final.
//! - Si esa ventana no llega al último mensaje, se pagina hacia lo más nuevo (`Anchor::After`) al
//!   acercarse al final, manteniendo lo que se ve en su sitio (el scroll está anclado abajo).
//! - Lo visible se marca como leído poco a poco: tras una espera sin moverse, `mark_read` con el
//!   mayor id que se ha visto, nunca todo de golpe al abrir.
//! - El botón «ir al final» lleva el contador de no leídos; si la ventana no llega al final, salta
//!   a los últimos mensajes.

use std::cell::Cell;
use std::collections::HashSet;
use std::future::Future;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::Id;
use iced::widget::operation::{self, AbsoluteOffset};
use tc_core::*;
use tc_theme::tokens::space;

use crate::app::{App, Backend, CONV_SCROLL_ID, Msg, error_text};
use crate::conv::{self, Conversation, Item, PAGE};

/// Tiempo sin moverse antes de marcar como leído lo visible.
pub const READ_DEBOUNCE: Duration = Duration::from_millis(350);
/// Cada cuánto se mira si toca marcar (solo mientras hay lectura pendiente).
pub const FLUSH_EVERY: Duration = Duration::from_millis(150);
/// Contexto ya leído que queda encima de la marca al abrir (px).
pub const UNREAD_CONTEXT: f32 = 96.0;
/// Un mensaje cuenta como visto cuando asoma al menos esto por abajo (px).
const SEEN_MARGIN: f32 = 24.0;

#[derive(Debug, Clone)]
pub enum ReadMsg {
    /// Página de mensajes más nuevos que la ventana (`Anchor::After`).
    Newer { peer: PeerRef, result: BackendResult<Vec<Message>> },
    /// Últimos mensajes («ir al final» con la ventana lejos del final).
    Latest { peer: PeerRef, result: BackendResult<Vec<Message>> },
    /// Tic de la espera del marcado como leído.
    Flush(Instant),
}

/// Estado de lectura de la conversación abierta.
#[derive(Debug)]
pub struct ReadState {
    /// La ventana en memoria llega hasta el último mensaje del chat.
    pub reached_end: bool,
    pub loading_newer: bool,
    /// Último leído con el que se pidió la primera página (`Anchor::Around`), si se abrió así.
    pub opened_at: Option<MsgId>,
    /// Falta colocar la vista en la marca «Sin leer desde aquí» (se hace en cuanto hay maqueta).
    pub jump_pending: bool,
    /// Mayor id (del servidor) con algo visible en pantalla; lo fija la vista en cada maquetación.
    pub seen: Cell<Option<MsgId>>,
    /// Mayor id enviado a `mark_read` desde que se abrió.
    pub sent: MsgId,
    /// Hay lectura pendiente desde este instante (se marca tras `READ_DEBOUNCE` sin moverse).
    pub dirty: Option<Instant>,
    /// No leídos y último leído al abrir (para estimar el contador mientras se lee a trozos).
    pub base_unread: u32,
    pub base_read: MsgId,
}

impl Default for ReadState {
    fn default() -> Self {
        Self {
            reached_end: true,
            loading_newer: false,
            opened_at: None,
            jump_pending: false,
            seen: Cell::new(None),
            sent: MsgId(0),
            dirty: None,
            base_unread: 0,
            base_read: MsgId(0),
        }
    }
}

/// Ancho de columna y alto de la vista de la conversación anterior: la nueva se maqueta igual, así
/// que el salto a la marca puede calcularse en cuanto llega la primera página.
#[derive(Debug, Clone, Copy)]
pub struct Carry {
    col: f32,
    viewport: f32,
}

impl Carry {
    pub fn of(c: &Conversation) -> Self {
        Self { col: c.last_col.get(), viewport: c.last_viewport.get() }
    }
}

// ---------------------------------------------------------------- funciones puras

/// Ancla de la primera página al abrir un chat.
pub fn initial_anchor(d: Option<&Dialog>) -> Anchor {
    match d {
        Some(d) if d.unread_count > 0 && d.read_inbox_max_id.0 > 0 => Anchor::Around(d.read_inbox_max_id),
        _ => Anchor::Latest,
    }
}

/// Primer entrante sin leer de la ventana. Sin `read_max` (0) se cuenta desde el final.
pub fn first_unread(msgs: &[Message], read_max: MsgId, unread: usize) -> Option<MsgId> {
    if unread == 0 {
        return None;
    }
    if read_max.0 > 0 {
        msgs.iter().find(|m| !m.outgoing && !conv::is_local(m) && m.id > read_max).map(|m| m.id)
    } else {
        msgs.iter().rev().filter(|m| !m.outgoing && !conv::is_local(m)).nth(unread - 1).map(|m| m.id)
    }
}

/// Desplazamiento (distancia al final, como lo cuenta el scroll anclado abajo) que deja el
/// elemento `idx` a `context` px del borde superior. Replica la maqueta de la vista: relleno
/// arriba si el contenido no llena la vista y `bottom_pad` al final.
pub fn from_bottom_for(heights: &[f32], idx: usize, vh: f32, context: f32, bottom_pad: f32) -> f32 {
    let acc: f32 = heights.iter().sum();
    let filler = (vh - acc - bottom_pad).max(0.0);
    let total = filler + acc + bottom_pad;
    let top = filler + heights[..idx.min(heights.len())].iter().sum::<f32>();
    let max = (total - vh).max(0.0);
    (total - vh - (top - context)).clamp(0.0, max)
}

/// Altura de los elementos que van detrás del mensaje `id` (lo que crece por abajo al añadir).
pub fn height_after(items: &[Item], msgs: &[Message], id: MsgId) -> f32 {
    items
        .iter()
        .position(|it| matches!(it, Item::Msg { idx, .. } if msgs[*idx].id == id))
        .map_or(0.0, |p| items[p + 1..].iter().map(Item::height).sum())
}

/// Mayor id del servidor con al menos `SEEN_MARGIN` px dentro de la vista.
pub fn last_seen(items: &[Item], prefix: &[f32], msgs: &[Message], vh: f32, from_bottom: f32) -> Option<MsgId> {
    if items.is_empty() || vh <= SEEN_MARGIN {
        return None;
    }
    // Misma parte de arriba; el borde de abajo sube `SEEN_MARGIN`.
    let w = conv::visible_window(prefix, vh - SEEN_MARGIN, from_bottom + SEEN_MARGIN, 0.0);
    items[w.start..w.end]
        .iter()
        .filter_map(|it| match it {
            Item::Msg { idx, .. } if !conv::is_local(&msgs[*idx]) => Some(msgs[*idx].id),
            _ => None,
        })
        .max()
}

/// ¿Se muestra el botón «ir al final»?
pub fn show_jump(c: &Conversation) -> bool {
    c.from_bottom > 320.0 || c.new_below > 0 || !c.reading.reached_end
}

// ---------------------------------------------------------------- App

impl App {
    fn fetch<T, F, Fut>(&self, f: F, map: impl FnOnce(T) -> Msg + Send + 'static) -> Task<Msg>
    where
        F: FnOnce(Backend) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        match self.chat_backend() {
            Some(b) => Task::perform(f(b), map),
            None => Task::none(),
        }
    }

    fn scroll_to_offset(y: f32) -> Task<Msg> {
        operation::scroll_to(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(y) })
    }

    pub(crate) fn on_reading(&mut self, m: ReadMsg) -> Task<Msg> {
        match m {
            ReadMsg::Newer { peer, result } => self.on_newer(peer, result),
            ReadMsg::Latest { peer, result } => self.on_latest(peer, result),
            ReadMsg::Flush(now) => self.flush_read(now),
        }
    }

    /// Hay marcado como leído pendiente (activa el tic de `FLUSH_EVERY`).
    pub fn read_flush_due(&self) -> bool {
        self.conv.as_ref().is_some_and(|c| c.reading.dirty.is_some())
    }

    /// Número del botón «ir al final»: no leídos del chat, o los llegados mientras se leía arriba.
    pub fn jump_badge(&self, c: &Conversation) -> u32 {
        c.new_below.max(self.reading_dialog(c.peer).map_or(0, |d| d.unread_count))
    }

    /// Al abrir (ya creada la conversación nueva): ancla de la primera página y medidas heredadas.
    pub(crate) fn start_reading(&mut self, carry: Option<Carry>) -> Anchor {
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Anchor::Latest };
        // En un tema de foro, el «diálogo» es el tema (`app/forum.rs`).
        let anchor = initial_anchor(self.reading_dialog(peer).as_deref());
        let c = self.conv.as_mut().expect("comprobado");
        if let Some(k) = carry {
            c.last_col.set(k.col);
            c.last_viewport.set(k.viewport);
        }
        c.reading.opened_at = match anchor {
            Anchor::Around(r) => Some(r),
            _ => None,
        };
        anchor
    }

    /// Tras la primera página (`on_history`): marca de no leídos, bordes de la ventana y salto.
    pub(crate) fn after_first_page(&mut self, peer: PeerRef) -> Task<Msg> {
        let (unread, read_max, last) =
            self.reading_dialog(peer).map_or((0, MsgId(0), None), |d| (d.unread_count, d.read_inbox_max_id, d.last_message.as_ref().map(|m| m.id)));
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) else { return Task::none() };
        c.unread_from = first_unread(&c.messages, read_max, unread as usize);
        let newest = c.newest_real();
        let empty = c.messages.is_empty();
        let r = &mut c.reading;
        r.base_unread = unread;
        r.base_read = read_max;
        if r.opened_at.is_some() {
            // Ventana centrada: lo más antiguo se descubre pidiendo `Before` (vacía = principio) y
            // lo más nuevo se sabe por el último mensaje del diálogo.
            c.reached_start = empty;
            r.reached_end = empty || matches!((newest, last), (Some(n), Some(l)) if n >= l);
        } else {
            r.reached_end = true;
        }
        r.jump_pending = c.unread_from.is_some();
        // Lo que se vea en la primera maquetación cuenta como leído (tras la espera).
        r.dirty = Some(Instant::now());
        self.resolve_unread_jump()
    }

    /// Coloca la marca «Sin leer desde aquí» cerca del borde superior, si ya hay medidas.
    pub(crate) fn resolve_unread_jump(&mut self) -> Task<Msg> {
        let Some(c) = self.conv.as_ref().filter(|c| c.reading.jump_pending) else { return Task::none() };
        let (col, vh) = (c.last_col.get(), c.last_viewport.get());
        if col <= 0.0 || vh <= 0.0 || c.loading {
            return Task::none();
        }
        let (items, _) = crate::view::conversation::build_items(self, c, col);
        let heights: Vec<f32> = items.iter().map(Item::height).collect();
        let y = items.iter().position(|it| matches!(it, Item::UnreadMark)).map(|i| from_bottom_for(&heights, i, vh, UNREAD_CONTEXT, space::S2));
        let c = self.conv.as_mut().expect("comprobado");
        c.reading.jump_pending = false;
        c.reading.dirty = Some(Instant::now());
        match y {
            Some(y) => {
                c.from_bottom = y;
                Self::scroll_to_offset(y)
            }
            None => Task::none(),
        }
    }

    /// Tras cada movimiento del scroll (`on_conv_scrolled`, que ya fijó `from_bottom`).
    pub(crate) fn reading_on_scroll(&mut self, vh: f32) -> Task<Msg> {
        let jump = self.resolve_unread_jump();
        let Some(c) = self.conv.as_mut() else { return jump };
        if !c.reading.jump_pending {
            c.reading.dirty = Some(Instant::now());
        }
        let mut tasks = vec![jump];
        if !c.reading.reached_end
            && !c.reading.loading_newer
            && !c.loading
            && !c.reading.jump_pending
            && c.from_bottom < 1.5 * vh
            && let Some(newest) = c.newest_real()
        {
            c.reading.loading_newer = true;
            let peer = c.peer;
            tasks.push(self.fetch(
                move |b| async move { b.history(peer, Anchor::After(newest), PAGE).await },
                move |result| Msg::Reading(ReadMsg::Newer { peer, result }),
            ));
        }
        Task::batch(tasks)
    }

    /// Altura de lo que hay detrás del mensaje `id` con la maqueta actual (0 sin medidas).
    fn height_below(&self, id: MsgId) -> f32 {
        let Some(c) = &self.conv else { return 0.0 };
        let col = c.last_col.get();
        if col <= 0.0 {
            return 0.0;
        }
        let (items, _) = crate::view::conversation::build_items(self, c, col);
        height_after(&items, &c.messages, id)
    }

    fn on_newer(&mut self, peer: PeerRef, result: BackendResult<Vec<Message>>) -> Task<Msg> {
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) else { return Task::none() };
        c.reading.loading_newer = false;
        let page = match result {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %error_text(&e), "no se pudieron cargar los mensajes más nuevos");
                return Task::none();
            }
        };
        c.reading.reached_end = page.len() < PAGE;
        let Some(old_last) = c.newest_real() else { return Task::none() };
        let before = self.height_below(old_last);
        let senders: HashSet<Option<UserId>> = page.iter().map(|m| m.sender).collect();
        let c = self.conv.as_mut().expect("comprobado");
        let mut added = false;
        for m in page.into_iter().filter(|m| m.id > old_last) {
            added |= c.upsert(m);
        }
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().map(|s| self.want_user(s)).collect();
        if added {
            // Anclado abajo: lo añadido empujaría lo visible hacia arriba; se compensa.
            let delta = self.height_below(old_last) - before;
            let c = self.conv.as_mut().expect("comprobado");
            if delta > 0.0 {
                c.from_bottom += delta;
                tasks.push(Self::scroll_to_offset(c.from_bottom));
            }
        }
        Task::batch(tasks)
    }

    /// «Ir al final»: si la ventana no llega al último mensaje, se cargan los últimos.
    pub(crate) fn jump_to_latest(&mut self) -> Option<Task<Msg>> {
        let c = self.conv.as_mut().filter(|c| !c.reading.reached_end)?;
        c.reading.jump_pending = false;
        c.reading.loading_newer = true;
        let peer = c.peer;
        Some(self.fetch(
            move |b| async move { b.history(peer, Anchor::Latest, PAGE).await },
            move |result| Msg::Reading(ReadMsg::Latest { peer, result }),
        ))
    }

    fn on_latest(&mut self, peer: PeerRef, result: BackendResult<Vec<Message>>) -> Task<Msg> {
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) else { return Task::none() };
        c.reading.loading_newer = false;
        let page = match result {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %error_text(&e), "no se pudieron cargar los últimos mensajes");
                return Task::none();
            }
        };
        let senders: HashSet<Option<UserId>> = page.iter().map(|m| m.sender).collect();
        let pending: Vec<Message> = c.messages.iter().filter(|m| conv::is_local(m)).cloned().collect();
        c.reached_start = page.len() < PAGE;
        c.messages = page;
        c.messages.extend(pending);
        c.reading.reached_end = true;
        c.from_bottom = 0.0;
        c.new_below = 0;
        let newest = c.newest_real();
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().map(|s| self.want_user(s)).collect();
        tasks.push(Self::scroll_to_offset(0.0));
        if let Some(n) = newest {
            tasks.push(self.send_read(n));
        }
        Task::batch(tasks)
    }

    /// Mensaje nuevo del chat abierto. Devuelve `true` si no debe entrar en la ventana (no llega
    /// al final: se cuenta en «ir al final» y se verá al paginar o saltar).
    pub(crate) fn defer_new_message(&mut self, m: &Message) -> bool {
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == m.peer) else { return false };
        if !m.outgoing {
            c.reading.base_unread += 1;
        }
        if c.reading.reached_end {
            return false;
        }
        c.new_below += 1;
        true
    }

    /// Ajusta el contador de no leídos del chat abierto a lo ya marcado (la red solo lo pone a
    /// cero al leer hasta el final; a trozos se estima con la ventana en memoria).
    pub(crate) fn estimate_unread(&self, mut d: Dialog) -> Dialog {
        // En un tema, la fila del grupo no se estima con la lectura del tema.
        let Some(c) = self.conv.as_ref().filter(|c| c.peer == d.peer && self.open_thread().is_none()) else { return d };
        let r = &c.reading;
        let read = d.read_inbox_max_id.max(r.sent);
        if read <= r.base_read || d.unread_count == 0 {
            return d;
        }
        if r.reached_end && c.newest_real().is_some_and(|n| read >= n) {
            // Leído hasta el último mensaje del chat.
            d.unread_count = 0;
        } else {
            let done = c.messages.iter().filter(|m| !m.outgoing && !conv::is_local(m) && m.id > r.base_read && m.id <= read).count() as u32;
            d.unread_count = d.unread_count.min(r.base_unread.saturating_sub(done));
        }
        d.read_inbox_max_id = read;
        d
    }

    /// Envía `mark_read(max)` y actualiza ya el contador de la lista.
    fn send_read(&mut self, max: MsgId) -> Task<Msg> {
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        let peer = c.peer;
        c.reading.sent = c.reading.sent.max(max);
        self.forum_note_read(max);
        if let Some(i) = self.dialogs.iter().position(|d| d.peer == peer) {
            let d = self.estimate_unread(self.dialogs[i].clone());
            self.dialogs[i] = d;
        }
        self.fetch(move |b| async move { b.mark_read(peer, max).await }, Msg::Op)
    }

    /// Tic de la espera: marca como leído hasta lo último visto si ya pasó `READ_DEBOUNCE`.
    pub(crate) fn flush_read(&mut self, now: Instant) -> Task<Msg> {
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        let Some(t) = c.reading.dirty else { return Task::none() };
        if c.reading.jump_pending || now.saturating_duration_since(t) < READ_DEBOUNCE {
            return Task::none();
        }
        c.reading.dirty = None;
        let (peer, sent) = (c.peer, c.reading.sent);
        let Some(seen) = c.reading.seen.get() else { return Task::none() };
        let Some(d) = self.reading_dialog(peer) else { return Task::none() };
        if d.unread_count == 0 || seen <= d.read_inbox_max_id.max(sent) {
            return Task::none();
        }
        self.send_read(seen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desplazamiento_deja_la_marca_arriba_con_contexto() {
        // 100 elementos de 50 px; la vista mide 500 px.
        let h = vec![50.0; 100];
        // Marca en el elemento 60: su parte de arriba está a 3000 px; total 5000 + 8.
        let y = from_bottom_for(&h, 60, 500.0, 100.0, 8.0);
        // Arriba de la vista = 3000 - 100 → distancia al final = 5008 - 500 - 2900.
        assert!((y - 1608.0).abs() < 0.01, "{y}");
        // Cerca del final: no pasa del final.
        assert_eq!(from_bottom_for(&h, 99, 500.0, 100.0, 8.0), 0.0);
        // Cerca del principio: no pasa del principio.
        assert!((from_bottom_for(&h, 0, 500.0, 100.0, 8.0) - (5008.0 - 500.0)).abs() < 0.01);
        // Contenido más corto que la vista: no hay nada que mover.
        assert_eq!(from_bottom_for(&[50.0; 3], 1, 500.0, 100.0, 8.0), 0.0);
    }

    fn msg(id: i32, outgoing: bool) -> Message {
        Message {
            id: MsgId(id),
            peer: PeerRef::User(UserId(1)),
            sender: Some(UserId(if outgoing { 0 } else { 1 })),
            date: Timestamp(1000 + id as i64),
            edit_date: None,
            outgoing,
            text: "x".into(),
            entities: vec![],
            reply_to: None,
            forwarded_from: None,
            media: None,
            sticker: None,
            reactions: vec![],
            random_id: None,
            state: SendState::Sent,
            ttl_secs: None,
        }
    }

    #[test]
    fn primer_no_leido_por_read_inbox_o_contando_desde_el_final() {
        let v: Vec<Message> = (1..=10).map(|i| msg(i, i == 6)).collect();
        assert_eq!(first_unread(&v, MsgId(5), 4), Some(MsgId(7)), "salta los salientes");
        assert_eq!(first_unread(&v, MsgId(5), 0), None);
        assert_eq!(first_unread(&v, MsgId(0), 2), Some(MsgId(9)));
        assert_eq!(first_unread(&v, MsgId(10), 3), None, "ya leído todo lo de la ventana");
    }

    #[test]
    fn visto_y_altura_detras() {
        let msgs: Vec<Message> = (1..=20).map(|i| msg(i, false)).collect();
        let items: Vec<Item> =
            (0..20).map(|idx| Item::Msg { idx, pos: crate::style::GroupPos::Single, gap: 0.0, avatar: false, height: 50.0 }).collect();
        let prefix: Vec<f32> = (0..=20).map(|i| i as f32 * 50.0).collect();
        // Al final: se ve el último.
        assert_eq!(last_seen(&items, &prefix, &msgs, 200.0, 0.0), Some(MsgId(20)));
        // 300 px por encima del final: lo último visible es el 14 (el 15 empieza a 700 = borde).
        assert_eq!(last_seen(&items, &prefix, &msgs, 200.0, 300.0), Some(MsgId(14)));
        assert_eq!(height_after(&items, &msgs, MsgId(18)), 100.0);
        assert_eq!(height_after(&items, &msgs, MsgId(99)), 0.0);
    }
}
