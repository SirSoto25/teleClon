//! Stickers de la cuenta en la app: packs con el propio de Parlo como pack local adicional,
//! recientes y favoritos del servidor, imágenes de los stickers de la red (selector y burbujas) y
//! envío del pack propio cuando la cuenta no lo tiene (como archivo WebP).

use super::*;
use crate::sticker_img;

impl App {
    /// Recientes y favoritos del servidor (en el mock, vacíos).
    pub(crate) fn load_net_stickers(&self) -> Task<Msg> {
        self.call(
            |b| async move {
                let recent = b.recent_stickers().await;
                let faved = b.faved_stickers().await;
                (recent, faved)
            },
            |(recent, faved)| Msg::NetStickers { recent, faved },
        )
    }

    pub(crate) fn on_net_stickers(&mut self, recent: BackendResult<Vec<Sticker>>, faved: BackendResult<Vec<Sticker>>) -> Task<Msg> {
        if let Ok(r) = recent {
            self.sticker_lists.recent = r;
        }
        if let Ok(f) = faved {
            self.sticker_lists.faved = f;
        }
        Task::none()
    }

    /// Packs recibidos: se añade el propio si la cuenta no lo trae.
    pub(crate) fn on_sticker_sets(&mut self, result: BackendResult<Vec<StickerSet>>) -> Task<Msg> {
        match result {
            Ok(mut sets) => {
                self.sticker_lists.local_pack = stickers::with_local_pack(&mut sets);
                self.sticker_sets = sets;
            }
            Err(e) => {
                tracing::warn!(error = %e, "no se pudieron cargar los packs de stickers");
                if self.sticker_sets.is_empty() {
                    self.sticker_lists.local_pack = stickers::with_local_pack(&mut self.sticker_sets);
                }
            }
        }
        Task::none()
    }

    /// Sticker por id en los packs, recientes o favoritos.
    pub fn find_sticker(&self, id: StickerId) -> Option<&Sticker> {
        stickers::find_any(&self.sticker_sets, &self.sticker_lists, id)
    }

    /// Imagen descargada de un sticker de la red para dibujarlo a `size` px (o la otra variante si
    /// la buena aún no está). `None`: emoji de respaldo.
    pub fn sticker_image(&self, s: &Sticker, size: f32) -> Option<iced::widget::image::Handle> {
        let small = size <= sticker_img::SMALL_MAX;
        let want = sticker_img::variant_for(s, small)?;
        let other = if want == MediaVariant::Full { MediaVariant::Thumb } else { MediaVariant::Full };
        [want, other].into_iter().filter_map(|v| sticker_img::key(s, v)).find_map(|k| self.images.handle(&k))
    }

    /// Chat «neutro» para descargar stickers del selector (no puede ser un chat secreto).
    fn sticker_peer(&self) -> PeerRef {
        PeerRef::User(self.me_id().unwrap_or(UserId(0)))
    }

    /// Stickers de la red a la vista: burbujas de la ventana visible y, con el selector abierto, la
    /// pestaña actual y los iconos de las pestañas.
    pub(crate) fn sticker_requests(&self) -> Vec<ImageRequest> {
        let mut out = Vec::new();
        if let Some(cv) = &self.conv {
            let (a, z) = cv.image_range();
            for m in &cv.messages[a..z] {
                let Some(s) = m.sticker.as_ref() else { continue };
                // Un envío aún local no tiene mensaje del que refrescar la referencia: por el pack.
                let (peer, msg) = if conv::is_local(m) || matches!(cv.peer, PeerRef::Secret(_)) { (self.sticker_peer(), MsgId(0)) } else { (cv.peer, m.id) };
                out.extend(sticker_img::request(peer, msg, s, false));
            }
        }
        if self.picker.open {
            let peer = self.sticker_peer();
            for s in self.picker_tab_stickers() {
                out.extend(sticker_img::request(peer, MsgId(0), s, true));
            }
            for tab in self.picker_tabs() {
                let first = match tab {
                    stickers::Tab::Set(k) => self.sticker_sets.get(k).and_then(|s| s.stickers.first()),
                    _ => None,
                };
                out.extend(first.and_then(|s| sticker_img::request(peer, MsgId(0), s, true)));
            }
        }
        out
    }

    /// Envía `sticker`: por la red si existe allí; el pack propio añadido localmente, como WebP.
    pub(crate) fn sticker_send_task(&self, peer: PeerRef, sticker: Sticker, reply_to: Option<MsgId>, random_id: RandomId) -> Task<Msg> {
        let local = self.sticker_lists.local_pack && sticker.set_id == tc_stickers::pack::SET_ID;
        self.call(
            move |b| async move {
                if local { send_local_pack_sticker(b, peer, &sticker, reply_to, random_id).await } else { b.send_sticker(peer, sticker.id, reply_to, random_id).await }
            },
            move |result| Msg::Sent { peer, random_id, result },
        )
    }
}

/// Lado del WebP con el que se envía un sticker del pack propio (el tamaño de los stickers de la red).
const LOCAL_SEND_PX: u32 = 512;

/// El pack propio no existe en la red: se dibuja a 512 px, se guarda como WebP en un temporal y se
/// envía como archivo (`sticker.webp`). Cómo lo muestran los demás clientes NO está verificado.
async fn send_local_pack_sticker(b: Backend, peer: PeerRef, sticker: &Sticker, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
    let idx = tc_stickers::pack::index_of(sticker.id).ok_or(BackendError::NotFound)?;
    let frame = tc_stickers::pack::render(idx, LOCAL_SEND_PX).ok_or(BackendError::Internal)?;
    let bytes = tc_stickers::webp::encode_lossless(&frame).map_err(|_| BackendError::Internal)?;
    let path = std::env::temp_dir().join(format!("parlo-sticker-{}-{}.webp", sticker.id.0, random_id.0));
    std::fs::write(&path, &bytes).map_err(|e| BackendError::Storage(e.to_string()))?;
    let item = OutgoingMediaItem {
        kind: MediaKind::File,
        path: path.to_string_lossy().into_owned(),
        file_name: "sticker.webp".into(),
        size_bytes: bytes.len() as u64,
        width: frame.width,
        height: frame.height,
    };
    let out = OutgoingMedia { items: vec![item], caption: String::new(), reply_to, random_ids: vec![random_id] };
    let r = b.send_media(peer, out).await;
    let _ = std::fs::remove_file(&path);
    r?.into_iter().next().ok_or(BackendError::Internal)
}
