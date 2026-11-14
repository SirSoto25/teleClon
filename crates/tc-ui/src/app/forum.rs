//! Grupos con temas («comunidades»): al abrir un grupo foro se muestra la lista de temas (en el panel
//! de la conversación; en ventana estrecha, a pantalla completa) y al pulsar un tema se abre su
//! conversación con los componentes normales.
//!
//! Cómo se reutiliza la conversación: mientras hay un tema abierto, `App::call` y `chat_backend`
//! entregan un [`ThreadScope`] (tc-core) que lleva historial, envíos, lectura y «escribiendo…» del
//! grupo a ese tema; la lectura (`reading.rs`: abrir en el primer no leído, marcado progresivo) usa
//! [`App::reading_dialog`], que para un tema es el propio tema visto como diálogo. Los eventos de
//! temas (`TopicMessageNew`, `ForumTopicUpserted`) se tratan en [`App::forum_event`] antes del
//! `on_event` general.
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use iced::Task;
use iced::widget::Id;
use iced::widget::operation::{self, AbsoluteOffset};
use tc_core::*;

use crate::app::{App, Backend, LoadState, Msg, error_text};
use crate::format;

/// Temas por página.
pub const TOPICS_PAGE: usize = 50;
pub const TOPIC_SEARCH_ID: &str = "topic-search";
pub const TOPIC_NEW_ID: &str = "topic-new";
pub const TOPICS_SCROLL_ID: &str = "topics";

#[derive(Debug, Clone)]
pub enum ForumMsg {
    /// Página de temas (`append`: siguiente página; `query`: búsqueda del servidor).
    Loaded { group: GroupId, append: bool, query: Option<String>, result: BackendResult<TopicsPage> },
    OpenTopic(ThreadRef),
    /// Abre la fila `i` de la lista visible (guiones de capturas y atajos).
    OpenRow(usize),
    /// Menú contextual de la fila `i` de la lista visible, en el último punto del ratón.
    MenuRow(usize),
    /// De la conversación de un tema a la lista de temas.
    BackToTopics,
    /// Cierra la lista de temas (en estrecho, vuelve a la lista de chats).
    CloseList,
    Query(String),
    ClearQuery,
    Scrolled(iced::widget::scrollable::Viewport),
    Reload,
    OpenMenu(ThreadRef),
    CloseMenu,
    MarkRead(ThreadRef),
    Pin(ThreadRef, bool),
    Close(ThreadRef, bool),
    Done(BackendResult<()>),
    /// Nuevo tema: mostrar el campo, texto, crear y respuesta.
    StartNew,
    NewTitle(String),
    Create,
    Created(Box<BackendResult<ForumTopic>>),
}

/// Temas cargados de un foro.
#[derive(Debug, Clone)]
pub struct TopicList {
    pub items: Vec<ForumTopic>,
    pub state: LoadState,
    pub next: Option<TopicsOffset>,
    pub total: u32,
    pub loading_more: bool,
}

impl Default for TopicList {
    fn default() -> Self {
        Self { items: Vec::new(), state: LoadState::Loading, next: None, total: 0, loading_more: false }
    }
}

impl TopicList {
    fn upsert(&mut self, t: ForumTopic) {
        match self.items.iter_mut().find(|x| x.id == t.id) {
            Some(x) => *x = t,
            None => self.items.push(t),
        }
        sort_topics(&mut self.items);
    }
}

/// Estado de los foros en la UI (un campo de `App`).
#[derive(Debug, Default)]
pub struct ForumUi {
    /// Foro cuya lista de temas está abierta.
    pub list: Option<GroupId>,
    /// Tema cuya conversación está abierta (`App::conv` es su grupo).
    pub thread: Option<ThreadRef>,
    pub topics: HashMap<GroupId, TopicList>,
    /// Filtro por nombre de la lista abierta.
    pub query: String,
    /// Resultado de la búsqueda en el servidor (cuando la lista no está entera).
    pub found: Option<(String, Vec<ForumTopic>)>,
    /// Menú contextual de un tema, en el punto del clic (coordenadas de la ventana).
    pub menu: Option<(ThreadRef, iced::Point)>,
    /// Campo «Nuevo tema» visible y su texto (`None` = oculto).
    pub new_title: Option<String>,
    pub creating: bool,
    /// Se está abriendo un tema (la apertura normal no debe volver a la lista).
    opening: bool,
    /// Vida del `ThreadScope` del tema abierto: se apaga al salir para descartar respuestas tardías.
    alive: Option<Arc<AtomicBool>>,
}

impl ForumUi {
    fn kill_scope(&mut self) {
        if let Some(a) = self.alive.take() {
            a.store(false, Ordering::Release);
        }
    }
}

/// Primera letra del título (para el icono sin emoji).
pub fn topic_initial(title: &str) -> String {
    format::initials(title).chars().next().map(String::from).unwrap_or_else(|| "#".into())
}

impl App {
    pub fn is_forum(&self, peer: PeerRef) -> bool {
        matches!(peer, PeerRef::Group(g) if self.groups.get(&g).is_some_and(|g| g.is_forum))
    }

    /// Tema abierto ahora mismo (con su conversación a la vista).
    pub fn open_thread(&self) -> Option<ThreadRef> {
        self.forum.thread.filter(|t| self.conv.as_ref().is_some_and(|c| c.peer == t.peer()))
    }

    /// Lista de temas a la vista (sin conversación encima).
    pub fn topics_shown(&self) -> Option<GroupId> {
        self.forum.list.filter(|_| self.conv.is_none())
    }

    pub fn topic(&self, t: ThreadRef) -> Option<&ForumTopic> {
        self.forum.topics.get(&t.group)?.items.iter().find(|x| x.id == t.topic)
    }

    /// Temas visibles de la lista abierta: filtrados por nombre (y los encontrados en el servidor).
    pub fn visible_topics(&self, g: GroupId) -> Vec<&ForumTopic> {
        let Some(list) = self.forum.topics.get(&g) else { return Vec::new() };
        let q = format::fold(self.forum.query.trim());
        let mut out: Vec<&ForumTopic> = list.items.iter().filter(|t| !t.hidden || t.is_general()).filter(|t| format::matches(&t.title, &q)).collect();
        if let Some((fq, found)) = &self.forum.found
            && !q.is_empty()
            && format::fold(fq.trim()) == q
        {
            for t in found {
                if !out.iter().any(|x| x.id == t.id) {
                    out.push(t);
                }
            }
        }
        out
    }

    /// No leídos de una fila de la lista de chats: en un foro con temas cargados, la suma de sus temas.
    pub fn dialog_unread(&self, d: &Dialog) -> u32 {
        match d.peer {
            PeerRef::Group(g) if self.is_forum(d.peer) => match self.forum.topics.get(&g).filter(|l| l.state == LoadState::Ready && l.next.is_none()) {
                Some(l) => l.items.iter().map(|t| t.unread_count).sum(),
                None => d.unread_count,
            },
            _ => d.unread_count,
        }
    }

    /// ¿La fila del foro va marcada como seleccionada? (lista de temas o un tema suyo abiertos).
    pub fn forum_selected(&self, peer: PeerRef) -> bool {
        matches!(peer, PeerRef::Group(g) if self.forum.list == Some(g))
    }

    /// Diálogo con el que trabaja la lectura (`reading.rs`): en un tema, el tema como diálogo.
    pub fn reading_dialog(&self, peer: PeerRef) -> Option<Cow<'_, Dialog>> {
        if let Some(th) = self.forum.thread.filter(|t| t.peer() == peer)
            && let Some(t) = self.topic(th)
        {
            let title = self.dialog(peer).map(|d| d.title.clone()).unwrap_or_default();
            return Some(Cow::Owned(t.as_dialog(title)));
        }
        self.dialog(peer).map(Cow::Borrowed)
    }

    /// Backend para las llamadas: atado al tema si hay uno abierto.
    pub(crate) fn scoped_backend(&self) -> Option<Backend> {
        let b = self.backend.clone()?;
        match (self.open_thread(), &self.forum.alive) {
            (Some(th), Some(alive)) => Some(Arc::new(ThreadScope::new(b, th, alive.clone()))),
            _ => Some(b),
        }
    }

    /// Motivo por el que no se puede escribir en el tema abierto.
    pub(crate) fn forum_write_block(&self, peer: PeerRef) -> Option<String> {
        let th = self.open_thread().filter(|t| t.peer() == peer)?;
        self.topic(th).filter(|t| t.closed).map(|_| "Este tema está cerrado: solo escriben quienes administran el grupo.".into())
    }

    /// Al abrir un chat (`App::open`): un grupo foro abre su lista de temas; cualquier otro chat
    /// cierra la del foro. `Some` = ya resuelto aquí.
    pub(crate) fn forum_intercept_open(&mut self, peer: PeerRef) -> Option<Task<Msg>> {
        if self.forum.opening {
            return None;
        }
        let PeerRef::Group(g) = peer else {
            self.leave_forum();
            return None;
        };
        if !self.is_forum(peer) {
            self.leave_forum();
            return None;
        }
        let save = self.save_draft_task();
        self.forum.kill_scope();
        self.forum.thread = None;
        self.conv = None;
        self.picker.close();
        if self.forum.list != Some(g) {
            self.forum.query.clear();
            self.forum.found = None;
            self.forum.new_title = None;
        }
        self.forum.list = Some(g);
        self.forum.menu = None;
        Some(Task::batch([save, self.load_topics(g)]))
    }

    fn leave_forum(&mut self) {
        self.forum.kill_scope();
        self.forum.list = None;
        self.forum.thread = None;
        self.forum.menu = None;
        self.forum.new_title = None;
    }

    fn load_topics(&mut self, g: GroupId) -> Task<Msg> {
        let list = self.forum.topics.entry(g).or_default();
        if list.items.is_empty() {
            list.state = LoadState::Loading;
        }
        self.call(
            move |b| async move { b.forum_topics(g, TopicsQuery::first(TOPICS_PAGE)).await },
            move |result| Msg::Forum(ForumMsg::Loaded { group: g, append: false, query: None, result }),
        )
    }

    /// Mensajes de los foros. Ver `view/topics.rs` para la vista.
    pub(crate) fn on_forum(&mut self, m: ForumMsg) -> Task<Msg> {
        match m {
            ForumMsg::Loaded { group, append, query, result } => self.on_topics(group, append, query, result),
            ForumMsg::OpenTopic(th) => self.open_topic(th),
            ForumMsg::OpenRow(i) => match self.topics_shown().and_then(|g| self.visible_topics(g).get(i).map(|t| t.thread())) {
                Some(th) => self.open_topic(th),
                None => Task::none(),
            },
            ForumMsg::MenuRow(i) => {
                if let Some(th) = self.topics_shown().and_then(|g| self.visible_topics(g).get(i).map(|t| t.thread())) {
                    self.forum.menu = Some((th, self.chat_actions.cursor.get()));
                }
                Task::none()
            }
            ForumMsg::BackToTopics => {
                let Some(th) = self.open_thread() else { return Task::none() };
                self.forum.kill_scope();
                self.forum.thread = None;
                self.conv = None;
                self.picker.close();
                self.forum.list = Some(th.group);
                self.load_topics(th.group)
            }
            ForumMsg::CloseList => {
                self.leave_forum();
                Task::none()
            }
            ForumMsg::Query(q) => {
                self.forum.query = q;
                let q = self.forum.query.trim().to_string();
                let Some(g) = self.forum.list else { return Task::none() };
                // Con la lista entera basta el filtro local; si no, también se pregunta al servidor.
                let complete = self.forum.topics.get(&g).is_some_and(|l| l.next.is_none() && l.state == LoadState::Ready);
                if q.is_empty() || complete {
                    self.forum.found = None;
                    return Task::none();
                }
                let query = q.clone();
                self.call(
                    move |b| async move { b.forum_topics(g, TopicsQuery { query: Some(query), offset: None, limit: TOPICS_PAGE }).await },
                    move |result| Msg::Forum(ForumMsg::Loaded { group: g, append: false, query: Some(q), result }),
                )
            }
            ForumMsg::ClearQuery => {
                self.forum.query.clear();
                self.forum.found = None;
                Task::none()
            }
            ForumMsg::Scrolled(v) => {
                let Some(g) = self.topics_shown() else { return Task::none() };
                let near_end = v.absolute_offset_reversed().y < v.bounds().height;
                let Some(list) = self.forum.topics.get_mut(&g) else { return Task::none() };
                match list.next {
                    Some(offset) if near_end && !list.loading_more && list.state == LoadState::Ready => {
                        list.loading_more = true;
                        self.call(
                            move |b| async move { b.forum_topics(g, TopicsQuery { query: None, offset: Some(offset), limit: TOPICS_PAGE }).await },
                            move |result| Msg::Forum(ForumMsg::Loaded { group: g, append: true, query: None, result }),
                        )
                    }
                    _ => Task::none(),
                }
            }
            ForumMsg::Reload => match self.forum.list {
                Some(g) => self.load_topics(g),
                None => Task::none(),
            },
            ForumMsg::OpenMenu(th) => {
                self.forum.menu = Some((th, self.chat_actions.cursor.get()));
                Task::none()
            }
            ForumMsg::CloseMenu => {
                self.forum.menu = None;
                Task::none()
            }
            ForumMsg::MarkRead(th) => {
                self.forum.menu = None;
                self.notify_read(&super::notify::topic_key(th));
                let Some(max) = self.topic(th).and_then(|t| t.last_message.as_ref()).map(|m| m.id) else { return Task::none() };
                if let Some(t) = self.forum.topics.get_mut(&th.group).and_then(|l| l.items.iter_mut().find(|t| t.id == th.topic)) {
                    t.unread_count = 0;
                    t.unread_mentions = 0;
                    t.read_inbox_max_id = t.read_inbox_max_id.max(max);
                }
                self.call(move |b| async move { b.mark_topic_read(th, max).await }, |r| Msg::Forum(ForumMsg::Done(r)))
            }
            ForumMsg::Pin(th, pinned) => {
                self.forum.menu = None;
                if let Some(l) = self.forum.topics.get_mut(&th.group)
                    && let Some(mut t) = l.items.iter().find(|t| t.id == th.topic).cloned()
                {
                    t.pinned = pinned;
                    l.upsert(t);
                }
                self.call(move |b| async move { b.set_topic_pinned(th, pinned).await }, |r| Msg::Forum(ForumMsg::Done(r)))
            }
            ForumMsg::Close(th, closed) => {
                self.forum.menu = None;
                self.call(move |b| async move { b.set_topic_closed(th, closed).await }, |r| Msg::Forum(ForumMsg::Done(r)))
            }
            ForumMsg::Done(Ok(())) => Task::none(),
            ForumMsg::Done(Err(e)) => {
                self.toast(topic_error_text(&e), true);
                // Lo aplicado en pantalla puede no ser cierto: se recarga.
                match self.forum.list {
                    Some(g) => self.load_topics(g),
                    None => Task::none(),
                }
            }
            ForumMsg::StartNew => {
                self.forum.new_title = if self.forum.new_title.is_some() { None } else { Some(String::new()) };
                operation::focus(Id::new(TOPIC_NEW_ID))
            }
            ForumMsg::NewTitle(t) => {
                if let Some(x) = &mut self.forum.new_title {
                    *x = t;
                }
                Task::none()
            }
            ForumMsg::Create => {
                let (Some(g), Some(title)) = (self.forum.list, self.forum.new_title.clone()) else { return Task::none() };
                let title = title.trim().to_string();
                if title.is_empty() || self.forum.creating {
                    return Task::none();
                }
                self.forum.creating = true;
                let rid = self.new_random_id();
                self.call(move |b| async move { b.create_topic(g, title, None, rid).await }, |r| Msg::Forum(ForumMsg::Created(Box::new(r))))
            }
            ForumMsg::Created(r) => {
                self.forum.creating = false;
                match *r {
                    Ok(t) => {
                        self.forum.new_title = None;
                        let th = t.thread();
                        self.forum.topics.entry(t.group).or_default().upsert(t);
                        self.open_topic(th)
                    }
                    Err(e) => {
                        self.toast(topic_error_text(&e), true);
                        Task::none()
                    }
                }
            }
        }
    }

    fn on_topics(&mut self, g: GroupId, append: bool, query: Option<String>, result: BackendResult<TopicsPage>) -> Task<Msg> {
        if let Some(q) = query {
            if let Ok(p) = result
                && format::fold(q.trim()) == format::fold(self.forum.query.trim())
            {
                self.forum.found = Some((q, p.topics));
            }
            return Task::none();
        }
        let list = self.forum.topics.entry(g).or_default();
        list.loading_more = false;
        match result {
            Ok(p) => {
                if append {
                    for t in p.topics {
                        list.upsert(t);
                    }
                } else {
                    list.items = p.topics;
                    sort_topics(&mut list.items);
                }
                list.next = p.next;
                list.total = p.total;
                list.state = LoadState::Ready;
                let senders: Vec<Option<UserId>> = list.items.iter().filter_map(|t| t.last_message.as_ref()).map(|m| m.sender).collect();
                Task::batch(senders.into_iter().map(|s| self.want_user(s)).collect::<Vec<_>>())
            }
            Err(e) => {
                if list.items.is_empty() {
                    list.state = LoadState::Failed(topic_error_text(&e));
                } else {
                    self.toast(topic_error_text(&e), true);
                }
                Task::none()
            }
        }
    }

    /// Abre la conversación de un tema (la lista sigue detrás para volver).
    pub(crate) fn open_topic(&mut self, th: ThreadRef) -> Task<Msg> {
        self.forum.menu = None;
        if self.open_thread() == Some(th) {
            return Task::none();
        }
        let save = self.save_draft_task();
        self.forum.kill_scope();
        self.forum.alive = Some(Arc::new(AtomicBool::new(true)));
        self.forum.list = Some(th.group);
        self.forum.thread = Some(th);
        // Otro tema del mismo grupo: la conversación anterior se cierra antes de abrir.
        self.conv = None;
        self.forum.opening = true;
        let open = self.open(th.peer());
        self.forum.opening = false;
        Task::batch([save, open, operation::scroll_to(Id::new(crate::app::CONV_SCROLL_ID), AbsoluteOffset { x: None, y: Some(0.0) })])
    }

    /// Lectura enviada en el tema abierto (`reading.rs::send_read`): el contador del tema baja ya.
    pub(crate) fn forum_note_read(&mut self, max: MsgId) {
        let Some(th) = self.open_thread() else { return };
        let done = self.conv.as_ref().map_or(0, |c| {
            let base = self.topic(th).map_or(MsgId(0), |t| t.read_inbox_max_id);
            c.messages.iter().filter(|m| !m.outgoing && !crate::conv::is_local(m) && m.id > base && m.id <= max).count() as u32
        });
        if let Some(t) = self.forum.topics.get_mut(&th.group).and_then(|l| l.items.iter_mut().find(|t| t.id == th.topic)) {
            if max > t.read_inbox_max_id {
                t.unread_count = t.unread_count.saturating_sub(done);
                t.read_inbox_max_id = max;
            }
            if t.last_message.as_ref().is_some_and(|l| l.id <= max) {
                t.unread_count = 0;
            }
        }
    }

    /// Eventos de temas, antes del `on_event` general. `Ok` = tratado aquí; `Err` = sigue su curso.
    pub(crate) fn forum_event(&mut self, ev: BackendEvent) -> Result<Task<Msg>, Box<BackendEvent>> {
        let thread = self.open_thread();
        match ev {
            BackendEvent::TopicMessageNew { topic, msg } => {
                let PeerRef::Group(g) = msg.peer else { return Err(Box::new(BackendEvent::MessageNew(msg))) };
                if thread == Some(ThreadRef { group: g, topic }) {
                    // El tema abierto: entra en la conversación como cualquier mensaje nuevo.
                    return Err(Box::new(BackendEvent::MessageNew(msg)));
                }
                if self.conv.as_ref().is_some_and(|c| c.peer == msg.peer) && thread.is_none() {
                    // El foro abierto como chat plano (backend sin temas): se ve todo.
                    return Err(Box::new(BackendEvent::MessageNew(msg)));
                }
                Ok(self.want_user(msg.sender))
            }
            BackendEvent::ForumTopicUpserted(t) => {
                let want = self.want_user(t.last_message.as_ref().and_then(|m| m.sender));
                if let Some(l) = self.forum.topics.get_mut(&t.group) {
                    // En el tema abierto se respeta lo ya leído aquí (la red puede ir por detrás).
                    let mut t = t;
                    if thread == Some(t.thread())
                        && let Some(old) = l.items.iter().find(|x| x.id == t.id)
                        && old.read_inbox_max_id > t.read_inbox_max_id
                    {
                        t.read_inbox_max_id = old.read_inbox_max_id;
                        t.unread_count = t.unread_count.min(old.unread_count);
                    }
                    l.upsert(t);
                }
                Ok(want)
            }
            BackendEvent::ForumTopicsChanged { group } => {
                if self.forum.list == Some(group) {
                    return Ok(self.load_topics(group));
                }
                if let Some(l) = self.forum.topics.get_mut(&group) {
                    l.state = LoadState::Loading;
                }
                Ok(Task::none())
            }
            // En la conversación de un tema solo entra lo de ese tema.
            BackendEvent::MessageNew(m) if thread.is_some_and(|t| t.peer() == m.peer) => Ok(self.want_user(m.sender)),
            BackendEvent::MessageEdited(m) if thread.is_some_and(|t| t.peer() == m.peer) && self.conv.as_ref().is_some_and(|c| c.find(m.id).is_none()) => {
                Ok(Task::none())
            }
            BackendEvent::DialogRemoved { peer } => {
                if matches!(peer, PeerRef::Group(g) if self.forum.list == Some(g)) {
                    self.leave_forum();
                }
                Err(Box::new(BackendEvent::DialogRemoved { peer }))
            }
            other => Err(Box::new(other)),
        }
    }
}

/// Texto de error de las acciones sobre temas.
pub fn topic_error_text(e: &BackendError) -> String {
    match e {
        BackendError::Forbidden(r) if r == UNSUPPORTED => "Este backend todavía no admite temas.".into(),
        BackendError::Forbidden(r) if r.contains("TOPIC_CLOSED") => "Este tema está cerrado.".into(),
        BackendError::Forbidden(r) if r.contains("CHAT_ADMIN_REQUIRED") || r.contains("RIGHT_FORBIDDEN") => {
            "Hace falta ser administrador para hacer eso en el grupo.".into()
        }
        BackendError::InvalidInput("title") => "El título del tema no puede estar vacío (máximo 128 caracteres).".into(),
        _ => error_text(e),
    }
}
