//! Panel del compositor (Emojis | GIFs | Stickers): apertura perezosa, pestañas, inserción de
//! emojis en el cursor, recientes, búsqueda y envío de GIFs, y carga de las animaciones de los GIF
//! reales. Estado en `panel.rs` y `gifs.rs`; vistas en `view/panel.rs`.

use super::*;
use crate::emoji;
use crate::gifs::{self, GifMsg, Search};
use crate::panel::{GIF_SEARCH_ID, PanelMsg, PanelTab};

/// Documento de un GIF real a animar y de dónde descargarlo.
struct AnimTarget {
    peer: PeerRef,
    msg: MsgId,
    file: FileRef,
}

impl App {
    /// Se acaba de abrir el panel: se pide lo que aún no se tenga (uso de emojis, GIFs guardados).
    pub(crate) fn panel_opened(&mut self) -> Task<Msg> {
        let mut tasks = Vec::new();
        if !self.panel.emoji.usage_requested {
            self.panel.emoji.usage_requested = true;
            tasks.push(self.call(|b| async move { b.emoji_usage().await }, |r| Msg::Panel(PanelMsg::Usage(r))));
        }
        if self.panel.tab == PanelTab::Gifs {
            tasks.push(self.load_saved_gifs(false));
        }
        tasks.push(self.panel_focus());
        Task::batch(tasks)
    }

    /// Foco al abrir o cambiar de pestaña: el compositor (emojis y stickers) o el buscador de GIFs.
    fn panel_focus(&self) -> Task<Msg> {
        match self.panel.tab {
            PanelTab::Gifs => operation::focus(Id::new(GIF_SEARCH_ID)),
            _ => operation::focus(Id::new(COMPOSER_ID)),
        }
    }

    fn load_saved_gifs(&mut self, force: bool) -> Task<Msg> {
        let g = &mut self.panel.gifs;
        if g.saved_loading || (g.saved.is_some() && !force) {
            return Task::none();
        }
        g.saved_loading = true;
        g.saved_error = None;
        self.call(|b| async move { b.saved_gifs().await }, |r| Msg::Panel(PanelMsg::Gif(GifMsg::Saved(r))))
    }

    /// ¿Hay un cursor de teclado en la rejilla de la pestaña actual? (Enter lo activa).
    pub fn panel_cursor(&self) -> Option<usize> {
        match self.panel.tab {
            PanelTab::Stickers => self.picker.cursor,
            PanelTab::Gifs => self.panel.gifs.cursor.cursor,
            PanelTab::Emoji => None,
        }
    }

    /// Navegación con teclado fuera de la pestaña de stickers. `None`: la maneja el selector de
    /// stickers como siempre.
    pub(crate) fn panel_nav(&mut self, nav: Nav) -> Option<Task<Msg>> {
        match self.panel.tab {
            PanelTab::Stickers => None,
            PanelTab::Emoji => Some(Task::none()),
            PanelTab::Gifs => {
                let ids: Vec<i64> = self.panel.gifs.shown().iter().map(|g| g.id).collect();
                let cols = self.panel.gifs.cols.get();
                let i = self.panel.gifs.cursor.nav(nav, ids.len(), cols, 1);
                Some(match i.and_then(|i| ids.get(i)) {
                    Some(id) => self.on_gif(GifMsg::Send(*id)),
                    None => Task::none(),
                })
            }
        }
    }

    pub(crate) fn on_panel(&mut self, m: PanelMsg) -> Task<Msg> {
        match m {
            PanelMsg::Tab(tab) => {
                self.panel.tab = tab;
                self.picker.cursor = None;
                self.panel.gifs.cursor.cursor = None;
                if !self.picker.open {
                    return Task::none();
                }
                let load = if tab == PanelTab::Gifs { self.load_saved_gifs(false) } else { Task::none() };
                Task::batch([load, self.panel_focus()])
            }
            PanelMsg::Close => {
                if self.picker.open {
                    self.picker.close();
                    self.panel.emoji.tones_open = false;
                }
                Task::none()
            }
            PanelMsg::Category(c) => {
                self.panel.emoji.category = c;
                self.panel.emoji.query.clear();
                operation::scroll_to(Id::new(crate::view::emoji_panel::GRID_ID), AbsoluteOffset { x: None, y: Some(0.0) })
            }
            PanelMsg::EmojiQuery(q) => {
                self.panel.emoji.query = q;
                Task::none()
            }
            PanelMsg::EmojiSubmit => match emoji::search(&self.panel.emoji.query).first() {
                Some(e) => {
                    let s = emoji::apply(e, self.panel.emoji.tone).to_string();
                    self.insert_emoji(s)
                }
                None => Task::none(),
            },
            PanelMsg::Insert(e) => self.insert_emoji(e),
            PanelMsg::ToggleTones => {
                self.panel.emoji.tones_open = !self.panel.emoji.tones_open;
                Task::none()
            }
            PanelMsg::Tone(t) => {
                self.panel.emoji.tone = t;
                self.panel.emoji.tones_open = false;
                Task::none()
            }
            PanelMsg::Usage(r) => {
                match r {
                    Ok(list) => self.panel.emoji.usage.merge(list),
                    Err(e) => tracing::warn!(error = %e, "no se pudo leer el uso de emojis"),
                }
                Task::none()
            }
            PanelMsg::Recorded(r) => {
                if let Err(e) = r {
                    tracing::debug!(error = %e, "no se pudo guardar el uso de un emoji");
                }
                Task::none()
            }
            PanelMsg::Gif(g) => self.on_gif(g),
        }
    }

    /// Inserta `e` en la posición del cursor del compositor, apunta el uso y devuelve el foco al
    /// compositor (el panel sigue abierto).
    pub(crate) fn insert_emoji(&mut self, e: String) -> Task<Msg> {
        if self.conv.as_ref().is_none_or(|c| self.write_block(c.peer).is_some()) {
            return Task::none();
        }
        self.composer.perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(e.clone()))));
        let at = self::now();
        self.panel.emoji.usage.record(&e, at.0);
        let save = self.call(move |b| async move { b.record_emoji_use(e, at).await }, |r| Msg::Panel(PanelMsg::Recorded(r)));
        Task::batch([save, operation::focus(Id::new(COMPOSER_ID))])
    }

    fn on_gif(&mut self, m: GifMsg) -> Task<Msg> {
        match m {
            GifMsg::Saved(r) => {
                let g = &mut self.panel.gifs;
                g.saved_loading = false;
                match r {
                    Ok(list) => {
                        g.saved = Some(list);
                        g.saved_error = None;
                    }
                    Err(e) => {
                        g.saved_error = Some(error_text(&e));
                        g.saved.get_or_insert_with(Vec::new);
                    }
                }
                Task::none()
            }
            GifMsg::Query(q) => {
                self.panel.gifs.query = q;
                self.panel.gifs.cursor.cursor = None;
                Task::none()
            }
            GifMsg::Search => {
                let q = self.panel.gifs.query.trim().to_string();
                if q.is_empty() || self.panel.gifs.results().is_some_and(|s| s.loading || s.error.is_none()) {
                    return Task::none();
                }
                self.panel.gifs.search = Some(Search { query: q.clone(), loading: true, ..Default::default() });
                self.search_gifs_task(q, String::new())
            }
            GifMsg::More => {
                let Some(s) = self.panel.gifs.search.as_mut().filter(|s| !s.loading) else { return Task::none() };
                let Some(offset) = s.next.clone() else { return Task::none() };
                s.loading = true;
                let q = s.query.clone();
                self.search_gifs_task(q, offset)
            }
            GifMsg::Results { query, offset, result } => {
                let Some(s) = self.panel.gifs.search.as_mut().filter(|s| s.query == query) else { return Task::none() };
                s.loading = false;
                match result {
                    Ok(page) => {
                        if offset.is_empty() {
                            s.gifs = page.gifs;
                        } else {
                            s.gifs.extend(page.gifs);
                        }
                        s.next = page.next_offset;
                        s.error = None;
                    }
                    Err(e) => s.error = Some(error_text(&e)),
                }
                Task::none()
            }
            GifMsg::Retry => self.load_saved_gifs(true),
            GifMsg::Send(id) => match self.panel.gifs.find(id).cloned() {
                Some(g) => self.send_gif(g),
                None => Task::none(),
            },
            GifMsg::AnimLoaded(file, frames) => {
                self.panel.gifs.anims.finish(file, frames, Instant::now());
                Task::none()
            }
            GifMsg::Scrolled { y, h } => {
                (self.panel.gifs.scroll_y, self.panel.gifs.view_h) = (y, h);
                Task::none()
            }
        }
    }

    fn search_gifs_task(&self, query: String, offset: String) -> Task<Msg> {
        let (q, o) = (query.clone(), offset.clone());
        self.call(
            move |b| async move { b.search_gifs(q, o).await },
            move |result| Msg::Panel(PanelMsg::Gif(GifMsg::Results { query, offset, result })),
        )
    }

    /// Envío optimista de un GIF (pendiente con id local → confirmado por `random_id`). Cierra el panel.
    pub(crate) fn send_gif(&mut self, gif: Gif) -> Task<Msg> {
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Task::none() };
        if self.write_block(peer).is_some() || matches!(self.compose, Compose::Edit(_)) {
            return Task::none();
        }
        self.picker.close();
        let reply_to = match self.compose {
            Compose::Reply(r) => Some(r),
            _ => None,
        };
        let random_id = self.new_random_id();
        let ttl = match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| c.ttl_secs),
            _ => None,
        };
        let media = Media {
            id: gif.id,
            kind: MediaKind::Gif,
            file_name: Some(gifs::file_name(&gif)),
            size_bytes: gif.file.as_ref().map_or(0, |f| f.size_bytes),
            width: gif.width,
            height: gif.height,
            duration_secs: gif.duration_secs,
            local_path: None,
            file: gif.file.clone(),
            audio: None,
        };
        let pending = Message {
            id: MsgId(-self.local_seq),
            peer,
            sender: self.me_id(),
            date: self::now(),
            edit_date: None,
            outgoing: true,
            text: String::new(),
            entities: vec![],
            reply_to,
            forwarded_from: None,
            media: Some(media),
            sticker: None,
            reactions: vec![],
            random_id: Some(random_id),
            state: SendState::Pending,
            ttl_secs: ttl,
        };
        let now = Instant::now();
        let key = MsgKey::Random(random_id);
        self.anim.start(AnimKey::BubbleOpacity(key), 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), now);
        self.anim.start(AnimKey::BubbleY(key), 8.0, 0.0, Curve::new(Spring::ESTANDAR, Property::Position, 180), now);
        if let Some(c) = self.conv.as_mut() {
            c.push_pending(pending);
            c.from_bottom = 0.0;
            c.new_below = 0;
        }
        if reply_to.is_some() {
            self.compose = Compose::Normal;
        }
        self.panel.gifs.pending.insert(random_id, gif.clone());
        let send = self.gif_send_task(peer, gif, reply_to, random_id);
        Task::batch([send, Self::scroll_bottom(), operation::focus(Id::new(COMPOSER_ID))])
    }

    fn gif_send_task(&self, peer: PeerRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> Task<Msg> {
        self.call(move |b| async move { b.send_gif(peer, gif, reply_to, random_id).await }, move |result| Msg::Sent { peer, random_id, result })
    }

    /// Reintento de un GIF fallido (`None`: ese envío no era un GIF).
    pub(crate) fn retry_gif(&mut self, rid: RandomId) -> Option<Task<Msg>> {
        let gif = self.panel.gifs.pending.get(&rid).cloned()?;
        let m = self.conv.as_mut()?.messages.iter_mut().find(|m| m.random_id == Some(rid) && conv::is_local(m))?;
        m.state = SendState::Pending;
        let (peer, reply_to) = (m.peer, m.reply_to);
        Some(self.gif_send_task(peer, gif, reply_to, rid))
    }

    // ------------------------------------------------------------ imágenes y animaciones

    /// Miniaturas de los GIFs del panel (con la pestaña abierta), en el orden en que se ven.
    pub(crate) fn gif_requests(&self) -> Vec<ImageRequest> {
        if !self.picker.open || self.panel.tab != PanelTab::Gifs {
            return Vec::new();
        }
        let peer = self.sticker_peer();
        self.panel
            .gifs
            .shown()
            .into_iter()
            .take(60)
            .filter_map(|g| g.file.clone().filter(|f| f.thumb.is_some()))
            .map(|file| ImageRequest::Media { peer, msg: MsgId(0), file, variant: MediaVariant::Thumb, aspect: None })
            .collect()
    }

    /// Miniatura de un GIF del panel (también sirve para el envío aún pendiente).
    pub fn gif_thumb(&self, file: i64) -> Option<iced::widget::image::Handle> {
        self.images.handle(&ImageKey::Thumb { peer: self.sticker_peer(), msg: MsgId(0), file })
    }

    /// GIFs a la vista que se animan: los de las filas visibles del panel y los de las burbujas
    /// visibles, como mucho `gifs::MAX_ANIMATED` (así todos caben en la caché a la vez).
    fn gif_anim_targets(&self) -> Vec<AnimTarget> {
        let mut out: Vec<AnimTarget> = Vec::new();
        let mut push = |peer, msg, file: &FileRef| {
            if file.size_bytes <= gifs::MAX_ANIM_BYTES && !out.iter().any(|t| t.file.id == file.id) {
                out.push(AnimTarget { peer, msg, file: file.clone() });
            }
        };
        if self.picker.open && self.panel.tab == PanelTab::Gifs {
            let peer = self.sticker_peer();
            let shown = self.panel.gifs.shown();
            for g in shown.get(self.panel.gifs.visible_range(shown.len())).unwrap_or_default() {
                if let Some(f) = &g.file {
                    push(peer, MsgId(0), f);
                }
            }
        }
        if let Some(cv) = &self.conv {
            let (a, z) = cv.image_range();
            for m in cv.messages.get(a..z).unwrap_or_default() {
                let Some(md) = m.media.as_ref().filter(|md| gifs::animates(md)) else { continue };
                let Some(f) = &md.file else { continue };
                let (peer, msg) = if conv::is_local(m) || matches!(cv.peer, PeerRef::Secret(_)) { (self.sticker_peer(), MsgId(0)) } else { (cv.peer, m.id) };
                push(peer, msg, f);
            }
        }
        out.truncate(gifs::MAX_ANIMATED);
        out
    }

    /// ¿Hay un GIF animándose a la vista? Usa el mismo reloj que los stickers (`sticker_anim::TICK`).
    pub fn gifs_animating(&self) -> bool {
        if self.anim.pref.is_reduced() || self.phase != Phase::Main || !self.sticker_anims.focused {
            return false;
        }
        self.gif_anim_targets().iter().any(|t| self.panel.gifs.anims.is_ready(t.file.id))
    }

    /// Fotograma actual de la animación de un documento (primer fotograma con movimiento reducido).
    pub fn gif_frame(&self, file: i64) -> Option<iced::widget::image::Handle> {
        let f = self.panel.gifs.anims.get(file)?;
        let ms = if self.anim.pref.is_reduced() { 0.0 } else { self.sticker_ms };
        f.at(ms).cloned()
    }

    /// Lanza la descarga y el decodificado de las animaciones a la vista que falten.
    pub(crate) fn pump_gif_anims(&mut self) -> Task<Msg> {
        let Some(backend) = self.backend.clone().filter(|_| self.phase == Phase::Main) else { return Task::none() };
        let now = Instant::now();
        let mut tasks = Vec::new();
        for t in self.gif_anim_targets() {
            if !self.panel.gifs.anims.can_start() {
                break;
            }
            if !self.panel.gifs.anims.wants(t.file.id, now) {
                continue;
            }
            let id = t.file.id;
            self.panel.gifs.anims.start(id);
            let b = backend.clone();
            tasks.push(Task::perform(
                async move {
                    let bytes = b.download_media(t.peer, t.msg, t.file, MediaVariant::Full).await.ok()?;
                    tokio::task::spawn_blocking(move || gifs::decode(&bytes).map(Arc::new)).await.ok().flatten()
                },
                move |frames| Msg::Panel(PanelMsg::Gif(GifMsg::AnimLoaded(id, frames))),
            ));
        }
        Task::batch(tasks)
    }
}
