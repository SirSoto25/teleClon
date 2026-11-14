//! Envío de archivos (fase A2, ADR-0011): clip y Ctrl+O con el selector nativo, arrastrar y soltar,
//! pegar (rutas o una imagen del portapapeles), diálogo de envío y burbujas pendientes con progreso,
//! cancelación y «Reintentar».
//!
//! Cada envío es una llamada a `send_media` (un álbum de hasta 10 elementos). La UI genera los
//! `random_ids` antes de llamar y deriva el id de la transferencia del primero
//! (`TransferId::upload_of`), así puede cancelar mientras espera. Hay una burbuja pendiente por
//! elemento (el `MessageSent` de cada uno la sustituye) y todas muestran el progreso del envío.
//! `Err(Cancelled)` quita las burbujas (no es un fallo); cualquier otro error las deja en «Reintentar».

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use iced::Task;
use iced::widget::image::Handle;
use iced::widget::{Id, operation, text_editor};
use tc_core::*;
use tc_theme::Spring;
use tc_theme::motion::Property;

use super::{AnimKey, App, COMPOSER_ID, Compose, Msg, error_text};
use crate::anim::Curve;
use crate::conv::{self, MsgKey};
use crate::files::{self, CAPTION_ID, MAX_CAPTION_CHARS, Picked};

#[derive(Debug, Clone)]
pub enum FilesMsg {
    /// Clip o Ctrl+O: abrir el selector nativo.
    Pick,
    /// Rutas elegidas (selector, pegado o soltadas).
    Picked(Vec<PathBuf>),
    /// Archivos leídos (y avisos de los que no se pudieron leer).
    Inspected { files: Vec<Picked>, errors: Vec<String> },
    /// Arrastrando archivos sobre la ventana (`true`) o ya fuera (`false`).
    Hover(bool),
    /// Un archivo soltado sobre la ventana (iced manda uno por archivo).
    Dropped(PathBuf),
    /// Ctrl+V en el compositor: se mira el portapapeles.
    Paste,
    /// Texto del portapapeles (`None`: no hay texto; se prueba con una imagen).
    PasteText(Option<String>),
    /// Imagen del portapapeles guardada como PNG (`Ok(None)`: no había imagen).
    PastedImage(Result<Option<PathBuf>, String>),
    /// Diálogo: edición del pie de foto.
    Caption(text_editor::Action),
    /// Diálogo: `true` = comprimir fotos, `false` = enviar como archivo.
    Compress(bool),
    /// Diálogo: quitar un archivo de la lista.
    Remove(usize),
    /// Diálogo: cerrar sin enviar.
    Close,
    /// Diálogo: enviar.
    Send,
    /// Respuesta de `send_media` (`first` = primer `random_id` del envío).
    Sent { peer: PeerRef, first: RandomId, result: BackendResult<Vec<Message>> },
    /// Cancelar el envío en curso (cualquiera de sus burbujas).
    Cancel(RandomId),
}

/// Diálogo de envío abierto.
pub struct SendDialog {
    pub peer: PeerRef,
    /// Tema del foro en el que se abrió (el envío va a ese tema).
    pub thread: Option<ThreadRef>,
    pub files: Vec<Picked>,
    pub caption: text_editor::Content,
    /// `true`: las imágenes van como fotos comprimidas; `false`: como archivos (sin pérdida).
    pub compress: bool,
}

impl SendDialog {
    pub fn caption_text(&self) -> String {
        self.caption.text().trim().to_string()
    }

    pub fn caption_too_long(&self) -> bool {
        files::caption_len(&self.caption.text()) > MAX_CAPTION_CHARS
    }

    /// ¿Hay imágenes que se puedan comprimir? (Si no, no se ofrece la opción.)
    pub fn has_photos(&self) -> bool {
        self.files.iter().any(Picked::can_be_photo)
    }
}

/// Un envío en curso o fallido.
#[derive(Debug, Clone)]
pub struct Upload {
    pub peer: PeerRef,
    pub media: OutgoingMedia,
    pub done: u64,
    pub total: u64,
    /// Ya llegó algún evento de progreso (si no, se muestra «Enviando…» sin porcentaje).
    pub progress_known: bool,
    pub cancelling: bool,
    pub failed: bool,
}

impl Upload {
    pub fn fraction(&self) -> Option<f32> {
        (self.progress_known && self.total > 0).then(|| (self.done.min(self.total) as f64 / self.total as f64) as f32)
    }
}

#[derive(Default)]
pub struct Uploads {
    pub dialog: Option<SendDialog>,
    /// Envíos por su primer `random_id`.
    pub jobs: HashMap<RandomId, Upload>,
    /// `random_id` de cada elemento → primer `random_id` de su envío.
    pub owner: HashMap<RandomId, RandomId>,
    /// Vista previa local de cada elemento pendiente (fotos).
    pub previews: HashMap<RandomId, Handle>,
    /// Hay archivos arrastrándose sobre la ventana.
    pub hover: bool,
    /// El selector nativo está abierto (no se abre otro).
    pub picking: bool,
}

/// Eventos de ventana de arrastrar y soltar (para `iced::event::listen_with`).
pub fn drop_event(event: iced::Event, _status: iced::event::Status, _window: iced::window::Id) -> Option<Msg> {
    match event {
        iced::Event::Window(iced::window::Event::FileHovered(_)) => Some(Msg::Files(FilesMsg::Hover(true))),
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => Some(Msg::Files(FilesMsg::Hover(false))),
        iced::Event::Window(iced::window::Event::FileDropped(p)) => Some(Msg::Files(FilesMsg::Dropped(p))),
        _ => None,
    }
}

impl App {
    pub(crate) fn on_files(&mut self, m: FilesMsg) -> Task<Msg> {
        match m {
            FilesMsg::Pick => {
                if self.uploads.picking || self.attach_block().is_some() {
                    if let Some(r) = self.attach_block() {
                        self.toast(r, true);
                    }
                    return Task::none();
                }
                self.uploads.picking = true;
                Task::perform(files::pick_files(), |v| Msg::Files(FilesMsg::Picked(v)))
            }
            FilesMsg::Picked(paths) => {
                self.uploads.picking = false;
                Self::inspect_task(paths)
            }
            FilesMsg::Inspected { files, errors } => self.on_inspected(files, errors),
            FilesMsg::Hover(h) => {
                self.uploads.hover = h && self.conv.is_some();
                Task::none()
            }
            FilesMsg::Dropped(p) => {
                self.uploads.hover = false;
                if let Some(r) = self.attach_block() {
                    self.toast(r, true);
                    return Task::none();
                }
                Self::inspect_task(vec![p])
            }
            FilesMsg::Paste => iced::clipboard::read().map(|t| Msg::Files(FilesMsg::PasteText(t))),
            FilesMsg::PasteText(Some(t)) if !t.is_empty() => match files::paths_from_text(&t) {
                // Archivos copiados en el gestor de archivos: se adjuntan.
                Some(paths) if self.attach_block().is_none() => Self::inspect_task(paths),
                _ => {
                    self.composer.perform(text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(t))));
                    Task::none()
                }
            },
            FilesMsg::PasteText(_) if self.attach_block().is_none() => Task::perform(files::paste_image(), |r| Msg::Files(FilesMsg::PastedImage(r))),
            FilesMsg::PasteText(_) => Task::none(),
            FilesMsg::PastedImage(Ok(Some(p))) => Self::inspect_task(vec![p]),
            FilesMsg::PastedImage(Ok(None)) => Task::none(),
            FilesMsg::PastedImage(Err(e)) => {
                self.toast(e, true);
                Task::none()
            }
            FilesMsg::Caption(a) => {
                if let Some(d) = &mut self.uploads.dialog {
                    d.caption.perform(a);
                }
                Task::none()
            }
            FilesMsg::Compress(v) => {
                if let Some(d) = &mut self.uploads.dialog {
                    d.compress = v;
                }
                Task::none()
            }
            FilesMsg::Remove(i) => {
                if let Some(d) = &mut self.uploads.dialog
                    && i < d.files.len()
                {
                    d.files.remove(i);
                    if d.files.is_empty() {
                        return self.close_send_dialog();
                    }
                }
                Task::none()
            }
            FilesMsg::Close => self.close_send_dialog(),
            FilesMsg::Send => self.send_files(),
            FilesMsg::Sent { peer, first, result } => self.on_media_sent(peer, first, result),
            FilesMsg::Cancel(rid) => {
                let first = self.uploads.owner.get(&rid).copied().unwrap_or(rid);
                let Some(job) = self.uploads.jobs.get_mut(&first).filter(|j| !j.failed && !j.cancelling) else { return Task::none() };
                job.cancelling = true;
                let id = TransferId::Upload(first);
                self.call(move |b| async move { b.cancel_transfer(id).await }, Msg::Op)
            }
        }
    }

    /// Por qué no se puede adjuntar ahora (sin chat, chat de solo lectura o editando un mensaje).
    pub(crate) fn attach_block(&self) -> Option<String> {
        let Some(peer) = self.conv.as_ref().map(|c| c.peer) else { return Some("Abre un chat para enviar archivos.".into()) };
        if let Some(r) = self.write_block(peer) {
            return Some(r);
        }
        matches!(self.compose, Compose::Edit(_)).then(|| "Termina de editar el mensaje antes de adjuntar archivos.".into())
    }

    fn inspect_task(paths: Vec<PathBuf>) -> Task<Msg> {
        if paths.is_empty() {
            return Task::none();
        }
        Task::perform(files::inspect(paths), |(files, errors)| Msg::Files(FilesMsg::Inspected { files, errors }))
    }

    fn on_inspected(&mut self, picked: Vec<Picked>, errors: Vec<String>) -> Task<Msg> {
        if !errors.is_empty() {
            self.toast(errors.join(" "), true);
        }
        if picked.is_empty() {
            return Task::none();
        }
        if let Some(r) = self.attach_block() {
            self.toast(r, true);
            return Task::none();
        }
        let peer = self.conv.as_ref().map(|c| c.peer).expect("comprobado en attach_block");
        let thread = self.open_thread();
        if let Some(d) = self.uploads.dialog.as_mut().filter(|d| d.peer == peer && d.thread == thread) {
            // Más archivos sobre el diálogo abierto: se añaden (sin repetir).
            for p in picked {
                if !d.files.iter().any(|f| f.path == p.path) {
                    d.files.push(p);
                }
            }
            return Task::none();
        }
        // Lo escrito en el compositor pasa a ser el pie (vuelve si se cierra sin enviar).
        let text = self.composer.text().trim().to_string();
        self.composer = text_editor::Content::new();
        self.picker.close();
        self.uploads.dialog = Some(SendDialog { peer, thread, files: picked, caption: text_editor::Content::with_text(&text), compress: true });
        self.open_overlay_anim();
        Task::batch([operation::focus(Id::new(CAPTION_ID)), operation::move_cursor_to_end(Id::new(CAPTION_ID))])
    }

    fn close_send_dialog(&mut self) -> Task<Msg> {
        let Some(d) = self.uploads.dialog.take() else { return Task::none() };
        let caption = d.caption_text();
        if !caption.is_empty() && self.composer.text().trim().is_empty() {
            self.composer = text_editor::Content::with_text(&caption);
        }
        Task::batch([operation::focus(Id::new(COMPOSER_ID)), operation::move_cursor_to_end(Id::new(COMPOSER_ID))])
    }

    fn send_files(&mut self) -> Task<Msg> {
        let Some(d) = &self.uploads.dialog else { return Task::none() };
        let peer = d.peer;
        if self.conv.as_ref().is_none_or(|c| c.peer != peer) || self.open_thread() != d.thread {
            self.uploads.dialog = None;
            return Task::none();
        }
        if let Some(r) = self.write_block(peer) {
            self.toast(r, true);
            return Task::none();
        }
        if d.caption_too_long() {
            // Telegram lo rechazaría: se avisa y se conserva el texto para recortarlo.
            self.toast(format!("El pie de foto es demasiado largo: el máximo es {MAX_CAPTION_CHARS} caracteres."), true);
            return operation::focus(Id::new(CAPTION_ID));
        }
        if d.files.is_empty() {
            return Task::none();
        }
        let d = self.uploads.dialog.take().expect("comprobado");
        let caption = d.caption_text();
        let reply_to = match self.compose {
            Compose::Reply(r) => Some(r),
            _ => None,
        };
        if reply_to.is_some() {
            self.compose = Compose::Normal;
        }
        let ttl = match peer {
            PeerRef::Secret(s) => self.secrets.get(&s).and_then(|c| c.ttl_secs),
            _ => None,
        };
        let now = Instant::now();
        let mut tasks = Vec::new();
        for (gi, group) in files::plan(&d.files, d.compress).into_iter().enumerate() {
            let lead = gi == 0;
            let mut media = OutgoingMedia {
                items: Vec::new(),
                caption: if lead { caption.clone() } else { String::new() },
                reply_to: if lead { reply_to } else { None },
                random_ids: Vec::new(),
            };
            for (k, &i) in group.iter().enumerate() {
                let f = &d.files[i];
                let kind = f.send_kind(d.compress);
                let (w, h) = if kind == MediaKind::Photo { (f.width, f.height) } else { (0, 0) };
                let rid = self.new_random_id();
                let first_item = lead && k == 0;
                let pending = Message {
                    id: MsgId(-self.local_seq),
                    peer,
                    sender: self.me_id(),
                    date: super::now(),
                    edit_date: None,
                    outgoing: true,
                    text: if first_item { caption.clone() } else { String::new() },
                    entities: vec![],
                    reply_to: if first_item { reply_to } else { None },
                    forwarded_from: None,
                    media: Some(Media {
                        id: -i64::from(self.local_seq),
                        kind,
                        file_name: Some(f.name.clone()),
                        size_bytes: f.size,
                        width: w,
                        height: h,
                        duration_secs: None,
                        local_path: Some(f.path.to_string_lossy().into_owned()),
                        file: None,
                        audio: None,
                    }),
                    sticker: None,
                    reactions: vec![],
                    random_id: Some(rid),
                    state: SendState::Pending,
                    ttl_secs: ttl,
                };
                let key = MsgKey::Random(rid);
                self.anim.start(AnimKey::BubbleOpacity(key), 0.0, 1.0, Curve::new(Spring::ESTANDAR, Property::Opacity, 180), now);
                self.anim.start(AnimKey::BubbleY(key), 8.0, 0.0, Curve::new(Spring::ESTANDAR, Property::Position, 180), now);
                if let Some(c) = self.conv.as_mut() {
                    c.push_pending(pending);
                }
                if kind == MediaKind::Photo
                    && let Some(h) = &f.preview
                {
                    self.uploads.previews.insert(rid, h.clone());
                }
                media.items.push(OutgoingMediaItem {
                    kind,
                    path: f.path.to_string_lossy().into_owned(),
                    file_name: f.name.clone(),
                    size_bytes: f.size,
                    width: w,
                    height: h,
                });
                media.random_ids.push(rid);
            }
            let first = media.random_ids[0];
            for r in &media.random_ids {
                self.uploads.owner.insert(*r, first);
            }
            let total = media.items.iter().map(|i| i.size_bytes).sum();
            self.uploads.jobs.insert(first, Upload { peer, media: media.clone(), done: 0, total, progress_known: false, cancelling: false, failed: false });
            tasks.push(self.media_send_task(peer, first, media));
        }
        if let Some(c) = self.conv.as_mut() {
            c.from_bottom = 0.0;
            c.new_below = 0;
        }
        tasks.push(Self::scroll_bottom());
        tasks.push(operation::focus(Id::new(COMPOSER_ID)));
        Task::batch(tasks)
    }

    fn media_send_task(&self, peer: PeerRef, first: RandomId, media: OutgoingMedia) -> Task<Msg> {
        self.call(move |b| async move { b.send_media(peer, media).await }, move |result| Msg::Files(FilesMsg::Sent { peer, first, result }))
    }

    /// Quita las burbujas pendientes de un envío y lo olvida (cancelado o terminado).
    fn drop_upload(&mut self, first: RandomId) {
        let Some(job) = self.uploads.jobs.remove(&first) else { return };
        for r in &job.media.random_ids {
            self.uploads.owner.remove(r);
            self.uploads.previews.remove(r);
        }
        if let Some(c) = self.conv.as_mut().filter(|c| c.peer == job.peer) {
            c.messages.retain(|m| !(conv::is_local(m) && m.random_id.is_some_and(|r| job.media.random_ids.contains(&r))));
        }
    }

    fn on_media_sent(&mut self, peer: PeerRef, first: RandomId, result: BackendResult<Vec<Message>>) -> Task<Msg> {
        match result {
            Ok(msgs) => {
                if let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) {
                    for m in msgs {
                        c.upsert(m);
                    }
                }
                // Lo que no haya vuelto (no debería pasar) no se queda «enviando» para siempre.
                self.drop_upload(first);
            }
            Err(BackendError::Cancelled) => self.drop_upload(first),
            Err(e) => {
                let Some(job) = self.uploads.jobs.get_mut(&first) else { return Task::none() };
                job.failed = true;
                job.cancelling = false;
                let rids = job.media.random_ids.clone();
                if let Some(c) = self.conv.as_mut().filter(|c| c.peer == peer) {
                    for r in rids {
                        c.set_state(r, SendState::Failed);
                    }
                }
                self.toast(error_text(&e), true);
            }
        }
        Task::none()
    }

    /// Progreso de una subida (ADR-0011). Las descargas a disco las pinta la fase A3.
    pub(crate) fn on_transfer(&mut self, t: Transfer) -> Task<Msg> {
        let TransferId::Upload(first) = t.id else { return Task::none() };
        let Some(job) = self.uploads.jobs.get_mut(&first) else { return Task::none() };
        match t.state {
            TransferState::Running => {
                job.done = job.done.max(t.done);
                if t.total > 0 {
                    job.total = t.total;
                }
                job.progress_known = true;
            }
            TransferState::Done => {
                job.done = job.total;
                job.progress_known = true;
            }
            // El resultado de `send_media` (`Err(Cancelled)`) llega después: se quita ya.
            TransferState::Cancelled => self.drop_upload(first),
            // El error lo trae el resultado de `send_media` (que lo marca como fallido).
            TransferState::Failed(_) => {}
        }
        Task::none()
    }

    /// Reintento de un envío de archivos fallido (`None`: ese `random_id` no era de archivos).
    pub(crate) fn retry_upload(&mut self, rid: RandomId) -> Option<Task<Msg>> {
        let first = *self.uploads.owner.get(&rid)?;
        let job = self.uploads.jobs.get(&first)?;
        if !job.failed {
            return Some(Task::none());
        }
        let peer = job.peer;
        let c = self.conv.as_mut().filter(|c| c.peer == peer)?;
        // Lo que ya se confirmó (un álbum enviado a medias) no se repite.
        let alive: Vec<RandomId> =
            job.media.random_ids.iter().copied().filter(|r| c.messages.iter().any(|m| conv::is_local(m) && m.random_id == Some(*r))).collect();
        if alive.is_empty() {
            self.drop_upload(first);
            return Some(Task::none());
        }
        for r in &alive {
            c.set_state(*r, SendState::Pending);
        }
        let job = self.uploads.jobs.get_mut(&first)?;
        let keep: Vec<bool> = job.media.random_ids.iter().map(|r| alive.contains(r)).collect();
        let mut it = keep.iter();
        job.media.items.retain(|_| *it.next().unwrap_or(&false));
        job.media.random_ids.retain(|r| alive.contains(r));
        // El id de la transferencia sale del primer elemento que queda.
        let new_first = job.media.random_ids[0];
        let mut job = self.uploads.jobs.remove(&first)?;
        job.failed = false;
        job.cancelling = false;
        job.done = 0;
        job.progress_known = false;
        job.total = job.media.items.iter().map(|i| i.size_bytes).sum();
        for r in &job.media.random_ids {
            self.uploads.owner.insert(*r, new_first);
        }
        let media = job.media.clone();
        self.uploads.jobs.insert(new_first, job);
        Some(self.media_send_task(peer, new_first, media))
    }

    /// Envío al que pertenece una burbuja pendiente.
    pub fn upload_of(&self, m: &Message) -> Option<(RandomId, &Upload)> {
        if !conv::is_local(m) {
            return None;
        }
        let first = *self.uploads.owner.get(&m.random_id?)?;
        self.uploads.jobs.get(&first).map(|j| (first, j))
    }
}
