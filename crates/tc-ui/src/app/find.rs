//! Búsqueda dentro del chat abierto (fase B2, ADR-0012): la lupa de la cabecera y `Ctrl+F` abren
//! una barra con campo, contador «3 de 17», flechas y filtros por tipo.
//!
//! - Al teclear se espera `DEBOUNCE` sin cambios antes de preguntar al backend
//!   (`MessagesApi::search_in_chat`; con un tema abierto, el `ThreadScope` lo limita a ese tema).
//! - Los resultados llegan del más reciente al más antiguo; el primero se selecciona y se salta a él.
//!   ↑ / Enter van al anterior (más antiguo), ↓ / Mayús+Enter al siguiente; al llegar al final de lo
//!   cargado se pide la página siguiente con `SearchQuery::after(next)` y se avanza al llegar.
//! - Saltar a un resultado que no está en la ventana en memoria la sustituye por la de alrededor
//!   (`Anchor::Around`, como «abrir en el primer no leído»); la lectura sigue paginando desde ahí.
//! - `Esc` cierra la barra y deja la conversación como estaba (ventana y desplazamiento).
//! - Un resultado borrado mientras se navega (`MessagesDeleted`) sale de la lista.
//! - Los chats secretos no se buscan (solo existen en este dispositivo y el servidor no los conoce).

use std::ops::Range;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::Id;
use iced::widget::operation::{self, AbsoluteOffset};
use tc_core::*;
use tc_theme::tokens::space;

use crate::app::{App, COMPOSER_ID, CONV_SCROLL_ID, Msg, error_text};
use crate::conv::{self, Item, Run, PAGE};

/// Resultados por página.
pub const FIND_PAGE: usize = 50;
/// Espera sin teclear antes de buscar.
pub const DEBOUNCE: Duration = Duration::from_millis(280);
/// Cada cuánto se mira si venció la espera (solo mientras hay una pendiente).
pub const DEBOUNCE_TICK: Duration = Duration::from_millis(70);
/// Id del campo de búsqueda.
pub const FIELD_ID: &str = "chat-find";

#[derive(Debug, Clone)]
pub enum FindMsg {
    /// Lupa o `Ctrl+F`: abre la barra (o vuelve a enfocar el campo si ya está abierta).
    Open,
    /// Cierra la barra y restaura la posición previa.
    Close,
    Input(String),
    Filter(SearchFilter),
    /// Resultado anterior (más antiguo): ↑ y Enter.
    Older,
    /// Resultado siguiente (más reciente): ↓ y Mayús+Enter.
    Newer,
    /// Reintentar tras un error.
    Retry,
    /// Reloj de la espera del tecleo.
    Tick(Instant),
    /// Página de resultados (`seq` descarta respuestas de consultas ya sustituidas).
    Results { seq: u64, append: bool, result: BackendResult<SearchPage> },
    /// Ventana de historial alrededor del resultado al que se salta.
    Window { peer: PeerRef, target: MsgId, result: BackendResult<Vec<Message>> },
    /// Un campo de texto se quedó con `Esc` (el de la barra lo usa para perder el foco).
    EscCaptured,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FindState {
    /// Nada que buscar todavía (campo vacío y filtro «Todo»).
    Idle,
    Loading,
    Ready,
    Failed(String),
}

/// Conversación tal como estaba al abrir la barra (para `Esc`).
#[derive(Debug, Default)]
struct Saved {
    messages: Vec<Message>,
    from_bottom: f32,
    reached_start: bool,
    reached_end: bool,
    /// La ventana en memoria se sustituyó al saltar a un resultado no cargado.
    replaced: bool,
}

/// Estado de la barra de búsqueda del chat abierto.
#[derive(Debug)]
pub struct ChatSearch {
    pub peer: PeerRef,
    /// Tema en el que se busca (con un tema de foro abierto).
    pub thread: Option<ThreadRef>,
    pub text: String,
    pub filter: SearchFilter,
    /// Ids de los resultados, del más reciente al más antiguo.
    pub hits: Vec<MsgId>,
    pub total: Option<u32>,
    pub next: Option<MsgId>,
    /// Índice del resultado seleccionado en `hits`.
    pub current: Option<usize>,
    pub state: FindState,
    pub loading_more: bool,
    /// Se está cargando la ventana de este resultado.
    pub jumping: Option<MsgId>,
    /// Falta colocar la vista en este mensaje (aún no había medidas).
    scroll_pending: Option<MsgId>,
    /// Al llegar la página siguiente, avanzar al primero de ella.
    advance_after_more: bool,
    seq: u64,
    due: Option<Instant>,
    saved: Saved,
}

impl ChatSearch {
    /// Consulta vigente (sin paginar).
    pub fn query(&self) -> SearchQuery {
        SearchQuery::new(self.text.trim(), FIND_PAGE).with_filter(self.filter)
    }

    /// Generación de la consulta vigente (las respuestas de otra se descartan).
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// Hay una espera de tecleo pendiente (activa el reloj).
    pub fn debouncing(&self) -> bool {
        self.due.is_some()
    }

    /// Mensaje seleccionado.
    pub fn current_id(&self) -> Option<MsgId> {
        self.current.and_then(|i| self.hits.get(i).copied())
    }

    /// Total que se muestra: el del backend, o lo cargado (con «+» si hay más páginas).
    pub fn total_label(&self) -> String {
        match self.total {
            Some(t) => t.max(self.hits.len() as u32).to_string(),
            None if self.next.is_some() => format!("{}+", self.hits.len()),
            None => self.hits.len().to_string(),
        }
    }

    /// Contador «3 de 17» (`None` si no hay nada seleccionado).
    pub fn counter(&self) -> Option<String> {
        let i = self.current?;
        Some(format!("{} de {}", i + 1, self.total_label()))
    }

    /// ¿Hay un resultado más antiguo (cargado o por cargar)?
    pub fn has_older(&self) -> bool {
        self.current.map_or(!self.hits.is_empty(), |i| i + 1 < self.hits.len() || self.next.is_some())
    }

    pub fn has_newer(&self) -> bool {
        self.current.is_some_and(|i| i > 0)
    }

    /// Términos que se resaltan en las burbujas (texto de la consulta, si lo hay).
    pub fn terms(&self) -> Option<&str> {
        let t = self.text.trim();
        (!t.is_empty() && self.state != FindState::Idle).then_some(t)
    }
}

/// Etiqueta de cada filtro (chips de la barra).
pub fn filter_label(f: SearchFilter) -> &'static str {
    match f {
        SearchFilter::All => "Todo",
        SearchFilter::Photos => "Fotos",
        SearchFilter::Videos => "Vídeos",
        SearchFilter::Files => "Archivos",
        SearchFilter::Links => "Enlaces",
        SearchFilter::Voice => "Voz",
    }
}

pub const FILTERS: [SearchFilter; 6] =
    [SearchFilter::All, SearchFilter::Photos, SearchFilter::Videos, SearchFilter::Files, SearchFilter::Links, SearchFilter::Voice];

/// Filtro de la tecla `Alt+1`…`Alt+6`.
pub fn filter_from_digit(d: &str) -> Option<SearchFilter> {
    let i: usize = d.parse().ok()?;
    FILTERS.get(i.checked_sub(1)?).copied()
}

/// Texto propio para los errores de la búsqueda.
pub fn find_error_text(e: &BackendError) -> String {
    match e {
        BackendError::Network => "Sin conexión: no se puede buscar ahora.".into(),
        BackendError::Forbidden(_) => "En este chat no se puede buscar.".into(),
        BackendError::FloodWait(s) => format!("Demasiadas búsquedas seguidas; espera {s} s."),
        other => error_text(other),
    }
}

// ---------------------------------------------------------------- resaltado (funciones puras)

/// Tramos (en bytes de `text`) que coinciden con la consulta, con la misma regla que la búsqueda:
/// sin acentos ni mayúsculas; las palabras completas, y la última también como prefijo (solo se
/// resalta la parte que coincide).
///
/// Si la consulta tiene varias palabras y aparecen seguidas y en orden (la frase), se resalta solo
/// esa frase. Si no, se resalta cada palabra suelta (el servidor busca mensajes que las contengan
/// todas, en cualquier sitio).
pub fn highlight_ranges(query: &str, text: &str) -> Vec<Range<usize>> {
    let parts = |w: &str| -> Vec<String> { fold(w).split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_owned).collect() };
    let toks: Vec<String> = query.split_whitespace().flat_map(parts).collect();
    let Some((last, init)) = toks.split_last() else { return Vec::new() };
    // Palabras del texto: (inicio, fin, plegada).
    let mut words: Vec<(usize, usize, String)> = Vec::new();
    let mut start: Option<usize> = None;
    let ends = text.char_indices().map(|(i, c)| (i, c.is_alphanumeric())).chain(std::iter::once((text.len(), false)));
    for (i, alnum) in ends {
        if alnum {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            words.push((s, i, fold(&text[s..i])));
        }
    }
    // Fin (en bytes) de los `n` primeros caracteres plegados de la palabra `s..`.
    let prefix_end = |s: usize, e: usize, n: usize| -> usize {
        let (mut got, mut end) = (0, s);
        for (j, c) in text[s..e].char_indices() {
            if got >= n {
                break;
            }
            got += fold(c.encode_utf8(&mut [0; 4])).chars().count();
            end = s + j + c.len_utf8();
        }
        end
    };
    let last_n = last.chars().count();
    // Frase seguida: las anteriores exactas y la última como prefijo.
    let k = init.len();
    if k >= 1 {
        let mut phrase = Vec::new();
        for w in 0..words.len().saturating_sub(k) {
            if init.iter().enumerate().all(|(d, t)| words[w + d].2 == *t) && words[w + k].2.starts_with(last.as_str()) {
                let (s, e, _) = &words[w + k];
                phrase.extend(words[w..w + k].iter().map(|(s, e, _)| *s..*e));
                phrase.push(*s..prefix_end(*s, *e, last_n));
            }
        }
        if !phrase.is_empty() {
            return phrase;
        }
    }
    // Sin frase: cada palabra suelta.
    let mut out = Vec::new();
    for (s, e, f) in &words {
        if init.contains(f) || f == last {
            out.push(*s..*e);
        } else if f.starts_with(last.as_str()) {
            out.push(*s..prefix_end(*s, *e, last_n));
        }
    }
    out
}

/// Parte los tramos de estilo de un mensaje por los rangos resaltados. Devuelve cada tramo con
/// `true` si va resaltado. `runs` son trozos de `text` (como los da `conv::runs`).
pub fn split_runs<'a>(text: &'a str, runs: &[Run<'a>], ranges: &[Range<usize>]) -> Vec<(Run<'a>, bool)> {
    let base = text.as_ptr() as usize;
    let mut out = Vec::new();
    for r in runs {
        let off = (r.text.as_ptr() as usize).saturating_sub(base);
        let end = off + r.text.len();
        let mut cuts = vec![off, end];
        for g in ranges {
            for x in [g.start, g.end] {
                if x > off && x < end && r.text.is_char_boundary(x - off) {
                    cuts.push(x);
                }
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for w in cuts.windows(2) {
            let piece = Run { text: &r.text[w[0] - off..w[1] - off], ..*r };
            let hl = ranges.iter().any(|g| g.start <= w[0] && w[1] <= g.end);
            out.push((piece, hl));
        }
    }
    out
}

// ---------------------------------------------------------------- App

fn esc_captured(ev: iced::Event, status: iced::event::Status, _: iced::window::Id) -> Option<Msg> {
    match ev {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape), .. })
            if status == iced::event::Status::Captured =>
        {
            Some(Msg::Find(FindMsg::EscCaptured))
        }
        _ => None,
    }
}

impl App {
    /// Barra de búsqueda del chat (o tema) que está abierto ahora mismo.
    pub fn find_active(&self) -> Option<&ChatSearch> {
        let f = self.find.as_ref()?;
        let c = self.conv.as_ref()?;
        (c.peer == f.peer && self.open_thread() == f.thread).then_some(f)
    }

    fn find_mut(&mut self) -> Option<&mut ChatSearch> {
        let peer = self.conv.as_ref()?.peer;
        let thread = self.open_thread();
        self.find.as_mut().filter(|f| f.peer == peer && f.thread == thread)
    }

    /// ¿Se puede buscar en este chat? (No en los secretos.)
    pub fn can_find(&self, peer: PeerRef) -> bool {
        !matches!(peer, PeerRef::Secret(_))
    }

    /// Suscripciones de la barra: reloj de la espera y `Esc` capturado por un campo.
    pub(crate) fn find_subscriptions(&self) -> Vec<iced::Subscription<Msg>> {
        let Some(f) = self.find_active() else { return Vec::new() };
        let mut subs = vec![iced::event::listen_with(esc_captured)];
        if f.debouncing() {
            subs.push(iced::time::every(DEBOUNCE_TICK).map(|t| Msg::Find(FindMsg::Tick(t))));
        }
        subs
    }

    pub(crate) fn on_find(&mut self, m: FindMsg) -> Task<Msg> {
        match m {
            FindMsg::Open => self.find_open(),
            FindMsg::Close => self.find_close(),
            FindMsg::Input(s) => {
                let Some(f) = self.find_mut() else { return Task::none() };
                f.text = s;
                f.due = Some(Instant::now() + DEBOUNCE);
                Task::none()
            }
            FindMsg::Filter(filter) => {
                let Some(f) = self.find_mut() else { return Task::none() };
                if f.filter == filter {
                    return Task::none();
                }
                f.filter = filter;
                self.find_run()
            }
            FindMsg::Tick(now) => match self.find_mut() {
                Some(f) if f.due.is_some_and(|d| now >= d) => self.find_run(),
                _ => Task::none(),
            },
            FindMsg::Retry => self.find_run(),
            FindMsg::Older => self.find_step(true),
            FindMsg::Newer => self.find_step(false),
            FindMsg::Results { seq, append, result } => self.on_find_results(seq, append, result),
            FindMsg::Window { peer, target, result } => self.on_find_window(peer, target, result),
            FindMsg::EscCaptured => {
                // Solo si no hay nada por encima que deba cerrarse antes (esas capas tratan su `Esc`).
                let above = self.overlay.is_some() || self.uploads.dialog.is_some() || self.picker.open || self.chat_actions.menu.is_some();
                if self.find_active().is_some() && !above { self.find_close() } else { Task::none() }
            }
        }
    }

    fn find_open(&mut self) -> Task<Msg> {
        let Some(c) = self.conv.as_ref() else { return Task::none() };
        let peer = c.peer;
        if !self.can_find(peer) {
            self.toast("En los chats secretos no se puede buscar: sus mensajes solo están en este dispositivo.".into(), false);
            return Task::none();
        }
        if self.find_active().is_some() {
            return operation::focus(Id::new(FIELD_ID));
        }
        let saved = Saved {
            messages: c.messages.clone(),
            from_bottom: c.from_bottom,
            reached_start: c.reached_start,
            reached_end: c.reading.reached_end,
            replaced: false,
        };
        self.find = Some(ChatSearch {
            peer,
            thread: self.open_thread(),
            text: String::new(),
            filter: SearchFilter::All,
            hits: Vec::new(),
            total: None,
            next: None,
            current: None,
            state: FindState::Idle,
            loading_more: false,
            jumping: None,
            scroll_pending: None,
            advance_after_more: false,
            seq: 0,
            due: None,
            saved,
        });
        self.hover = None;
        self.reactions_for = None;
        operation::focus(Id::new(FIELD_ID))
    }

    /// Cierra la barra y deja la conversación como estaba al abrirla.
    pub(crate) fn find_close(&mut self) -> Task<Msg> {
        let active = self.find_active().is_some();
        let Some(f) = self.find.take() else { return Task::none() };
        if !active {
            return Task::none();
        }
        let saved = f.saved;
        let last = self.reading_dialog(f.peer).and_then(|d| d.last_message.as_ref().map(|m| m.id));
        let Some(c) = self.conv.as_mut() else { return Task::none() };
        if saved.replaced {
            let saved_newest = saved.messages.iter().rev().find(|m| !conv::is_local(m)).map(|m| m.id);
            let current: Vec<Message> = std::mem::take(&mut c.messages);
            // Lo llegado mientras se buscaba sigue a lo guardado si la ventana actual lo enlaza.
            let joins = saved.reached_end
                && saved_newest.is_some_and(|n| current.iter().find(|m| !conv::is_local(m)).is_some_and(|o| o.id <= n));
            let mut msgs: Vec<Message> = saved.messages.into_iter().filter(|m| !conv::is_local(m)).collect();
            if joins {
                let n = saved_newest.unwrap_or(MsgId(0));
                msgs.extend(current.iter().filter(|m| !conv::is_local(m) && m.id > n).cloned());
            }
            msgs.extend(current.into_iter().filter(conv::is_local));
            c.messages = msgs;
            c.reached_start = saved.reached_start;
            let newest = c.newest_real();
            c.reading.reached_end = saved.reached_end && (joins || c.new_below == 0) && match (newest, last) {
                (Some(n), Some(l)) => n >= l,
                _ => true,
            };
            c.loading_older = false;
        }
        c.from_bottom = saved.from_bottom;
        let y = c.from_bottom;
        let mut tasks = vec![operation::scroll_to(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(y) })];
        if self.write_block(f.peer).is_none() {
            tasks.push(operation::focus(Id::new(COMPOSER_ID)));
        }
        Task::batch(tasks)
    }

    /// Lanza la consulta vigente desde el principio (nuevo `seq`; lo anterior se descarta).
    fn find_run(&mut self) -> Task<Msg> {
        let Some(f) = self.find_mut() else { return Task::none() };
        f.due = None;
        f.seq += 1;
        f.hits.clear();
        f.total = None;
        f.next = None;
        f.current = None;
        f.loading_more = false;
        f.advance_after_more = false;
        f.jumping = None;
        let query = f.query();
        if query.is_empty() {
            f.state = FindState::Idle;
            return Task::none();
        }
        f.state = FindState::Loading;
        let (seq, peer) = (f.seq, f.peer);
        self.call(
            move |b| async move { b.search_in_chat(peer, query).await },
            move |result| Msg::Find(FindMsg::Results { seq, append: false, result }),
        )
    }

    fn find_more(&mut self) -> Task<Msg> {
        let Some(f) = self.find_mut() else { return Task::none() };
        let Some(next) = f.next.filter(|_| !f.loading_more) else { return Task::none() };
        f.loading_more = true;
        let (seq, peer, query) = (f.seq, f.peer, f.query().after(next));
        self.call(
            move |b| async move { b.search_in_chat(peer, query).await },
            move |result| Msg::Find(FindMsg::Results { seq, append: true, result }),
        )
    }

    fn on_find_results(&mut self, seq: u64, append: bool, result: BackendResult<SearchPage>) -> Task<Msg> {
        let Some(f) = self.find_mut().filter(|f| f.seq == seq) else { return Task::none() };
        let page = match result {
            Ok(p) => p,
            Err(e) => {
                let text = find_error_text(&e);
                if append {
                    f.loading_more = false;
                    f.advance_after_more = false;
                    self.toast(text, true);
                } else {
                    f.state = FindState::Failed(text);
                }
                return Task::none();
            }
        };
        f.state = FindState::Ready;
        f.next = page.next;
        f.total = page.total.or(f.total);
        let before = f.hits.len();
        for m in page.hits.iter().filter(|m| m.id.0 > 0) {
            if !f.hits.contains(&m.id) {
                f.hits.push(m.id);
            }
        }
        if append {
            f.loading_more = false;
            if std::mem::take(&mut f.advance_after_more) && f.hits.len() > before {
                f.current = Some(before);
                let id = f.hits[before];
                return self.find_jump(id);
            }
            return Task::none();
        }
        let senders: Vec<Option<UserId>> = page.hits.iter().map(|m| m.sender).collect();
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().map(|s| self.want_user(s)).collect();
        if let Some(f) = self.find_mut()
            && let Some(&first) = f.hits.first()
        {
            f.current = Some(0);
            tasks.push(self.find_jump(first));
        }
        Task::batch(tasks)
    }

    /// ↑/Enter (`older`) o ↓/Mayús+Enter.
    fn find_step(&mut self, older: bool) -> Task<Msg> {
        let Some(f) = self.find_mut() else { return Task::none() };
        if f.state == FindState::Loading {
            return Task::none();
        }
        let next = match (f.current, older) {
            (None, _) if !f.hits.is_empty() => Some(0),
            (Some(i), true) if i + 1 < f.hits.len() => Some(i + 1),
            (Some(_), true) if f.next.is_some() => {
                f.advance_after_more = true;
                return self.find_more();
            }
            (Some(i), false) if i > 0 => Some(i - 1),
            _ => None,
        };
        let Some(i) = next else { return Task::none() };
        f.current = Some(i);
        let id = f.hits[i];
        let more = if i + 1 == f.hits.len() { self.find_more() } else { Task::none() };
        Task::batch([self.find_jump(id), more])
    }

    /// Lleva la vista al mensaje `id`: si está en la ventana, desplaza; si no, carga la de alrededor.
    fn find_jump(&mut self, id: MsgId) -> Task<Msg> {
        let Some(c) = self.conv.as_ref() else { return Task::none() };
        let peer = c.peer;
        if c.find(id).is_some() {
            if let Some(f) = self.find_mut() {
                f.jumping = None;
            }
            return self.find_scroll(id);
        }
        let Some(f) = self.find_mut() else { return Task::none() };
        f.jumping = Some(id);
        self.call(
            move |b| async move { b.history(peer, Anchor::Around(id), PAGE).await },
            move |result| Msg::Find(FindMsg::Window { peer, target: id, result }),
        )
    }

    fn on_find_window(&mut self, peer: PeerRef, target: MsgId, result: BackendResult<Vec<Message>>) -> Task<Msg> {
        let last = self.reading_dialog(peer).and_then(|d| d.last_message.as_ref().map(|m| m.id));
        let Some(f) = self.find_mut().filter(|f| f.peer == peer && f.jumping == Some(target)) else { return Task::none() };
        f.jumping = None;
        let page = match result {
            Ok(p) => p,
            Err(e) => {
                self.toast(find_error_text(&e), true);
                return Task::none();
            }
        };
        if !page.iter().any(|m| m.id == target) {
            // El mensaje ya no existe (borrado entre la búsqueda y el salto).
            self.find_deleted(peer, &[target]);
            self.toast("Ese mensaje ya no existe.".into(), false);
            return Task::none();
        }
        f.saved.replaced = true;
        let full = page.len() >= PAGE;
        let senders: Vec<Option<UserId>> = page.iter().map(|m| m.sender).collect();
        let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) else { return Task::none() };
        let pending: Vec<Message> = c.messages.iter().filter(|m| conv::is_local(m)).cloned().collect();
        c.messages = page;
        c.messages.extend(pending);
        let newest = c.newest_real();
        // Lo más antiguo se descubre pidiendo `Before` (vacía = principio); lo más nuevo, por el
        // último mensaje del diálogo.
        c.reached_start = false;
        c.reading.reached_end = match (newest, last) {
            (Some(n), Some(l)) => n >= l,
            _ => !full,
        };
        c.reading.jump_pending = false;
        c.reading.dirty = Some(Instant::now());
        let mut tasks: Vec<Task<Msg>> = senders.into_iter().collect::<std::collections::HashSet<_>>().into_iter().map(|s| self.want_user(s)).collect();
        tasks.push(self.find_scroll(target));
        Task::batch(tasks)
    }

    /// Coloca el mensaje `id` hacia el centro de la vista (o lo deja pendiente sin medidas).
    fn find_scroll(&mut self, id: MsgId) -> Task<Msg> {
        let Some(c) = self.conv.as_ref() else { return Task::none() };
        let (col, vh) = (c.last_col.get(), c.last_viewport.get());
        if col <= 0.0 || vh <= 0.0 {
            if let Some(f) = self.find_mut() {
                f.scroll_pending = Some(id);
            }
            return Task::none();
        }
        let (items, _) = crate::view::conversation::build_items(self, c, col);
        let heights: Vec<f32> = items.iter().map(Item::height).collect();
        let Some(i) = items.iter().position(|it| matches!(it, Item::Msg { idx, .. } if c.messages[*idx].id == id)) else { return Task::none() };
        let context = ((vh - heights[i]) / 2.0).max(space::S4);
        let y = crate::reading::from_bottom_for(&heights, i, vh, context, space::S2);
        let c = self.conv.as_mut().expect("comprobado");
        c.from_bottom = y;
        if c.near_bottom() {
            c.new_below = 0;
        }
        if let Some(f) = self.find_mut() {
            f.scroll_pending = None;
        }
        operation::scroll_to(Id::new(CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(y) })
    }

    /// Tras cada movimiento del scroll: coloca un salto que esperaba medidas.
    pub(crate) fn find_on_scroll(&mut self) -> Task<Msg> {
        match self.find_active().and_then(|f| f.scroll_pending) {
            Some(id) => self.find_scroll(id),
            None => Task::none(),
        }
    }

    /// Mensajes borrados: salen de los resultados (y de la conversación guardada para `Esc`).
    pub(crate) fn find_deleted(&mut self, peer: PeerRef, ids: &[MsgId]) {
        let Some(f) = self.find.as_mut().filter(|f| f.peer == peer) else { return };
        f.saved.messages.retain(|m| conv::is_local(m) || !ids.contains(&m.id));
        let cur = f.current_id();
        let removed_before = f.current.map_or(0, |i| f.hits[..i].iter().filter(|h| ids.contains(h)).count());
        let before = f.hits.len();
        f.hits.retain(|h| !ids.contains(h));
        let gone = (before - f.hits.len()) as u32;
        if gone == 0 {
            return;
        }
        f.total = f.total.map(|t| t.saturating_sub(gone));
        f.current = match (f.current, cur) {
            (_, _) if f.hits.is_empty() => None,
            (Some(i), Some(id)) if !ids.contains(&id) => Some(i - removed_before),
            // El seleccionado se borró: queda seleccionado el siguiente más antiguo (sin saltar).
            (Some(i), _) => Some((i - removed_before).min(f.hits.len() - 1)),
            (None, _) => None,
        };
        if f.jumping.is_some_and(|j| ids.contains(&j)) {
            f.jumping = None;
        }
    }
}
