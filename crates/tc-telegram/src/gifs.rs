//! GIFs: guardados de la cuenta (`messages.getSavedGifs` con `hash`), búsqueda con el bot inline
//! `@gif` (`contacts.resolveUsername` una vez + `messages.getInlineBotResults`) y envío.
//!
//! * Un GIF guardado se envía como documento (`messages.sendMedia` con `InputMediaDocument`); si su
//!   `file_reference` caducó se vuelven a pedir los guardados (sin `hash`) y se reintenta una vez.
//! * Un resultado de búsqueda se envía con `messages.sendInlineBotResult` (`hide_via`, como los
//!   clientes oficiales); si la consulta caducó (`QUERY_ID_INVALID`…) se envía su documento.
//! * Solo se usan los resultados con documento (`botInlineMediaResult`); los de URL externa se
//!   ignoran (necesitarían `upload.getWebFile`). Ver docs/adr/0010-gifs.md.
use std::collections::HashMap;

use grammers_tl_types as tl;
use tc_core::*;

use crate::convert::document_file_ref;

/// Usuario del bot de búsqueda de GIFs de Telegram.
pub(crate) const GIF_BOT: &str = "gif";

/// Lo que se sabe de los GIFs en esta sesión.
#[derive(Default)]
pub(crate) struct GifCatalog {
    /// `hash` de `messages.savedGifs` y la última lista.
    saved: (i64, Vec<Gif>),
    /// Documentos vistos (guardados o de búsquedas) para enviarlos.
    docs: HashMap<i64, tl::types::InputDocument>,
    /// Id del bot `@gif` ya resuelto.
    bot: Option<i64>,
}

/// GIF de un documento (MP4 sin sonido o GIF real).
pub(crate) fn gif_from_document(d: &tl::types::Document) -> Gif {
    let (mut w, mut h, mut dur, mut name) = (0u32, 0u32, None, String::new());
    for a in &d.attributes {
        match a {
            tl::enums::DocumentAttribute::ImageSize(s) => (w, h) = (s.w.max(0) as u32, s.h.max(0) as u32),
            tl::enums::DocumentAttribute::Video(v) => {
                (w, h) = (v.w.max(0) as u32, v.h.max(0) as u32);
                dur = Some(v.duration.max(0.0).round() as u32);
            }
            tl::enums::DocumentAttribute::Filename(f) => name = f.file_name.clone(),
            _ => {}
        }
    }
    Gif { id: d.id, file: Some(document_file_ref(d)), mime: d.mime_type.clone(), width: w, height: h, duration_secs: dur, title: name, inline: None }
}

fn input_doc(d: &tl::types::Document) -> tl::types::InputDocument {
    tl::types::InputDocument { id: d.id, access_hash: d.access_hash, file_reference: d.file_reference.clone() }
}

impl crate::Inner {
    fn gif_catalog(&self) -> std::sync::MutexGuard<'_, GifCatalog> {
        self.gifs.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Traduce y recuerda documentos de GIF.
    fn remember_gifs(&self, docs: impl IntoIterator<Item = tl::types::Document>) -> Vec<Gif> {
        let mut cat = self.gif_catalog();
        docs.into_iter()
            .map(|d| {
                cat.docs.insert(d.id, input_doc(&d));
                gif_from_document(&d)
            })
            .collect()
    }

    /// GIFs guardados (pide con el `hash` de la última vez: sin cambios, no baja nada).
    pub(crate) async fn saved_gifs(&self) -> BackendResult<Vec<Gif>> {
        self.ensure_logged_in()?;
        let hash = self.gif_catalog().saved.0;
        self.fetch_saved_gifs(hash).await
    }

    async fn fetch_saved_gifs(&self, hash: i64) -> BackendResult<Vec<Gif>> {
        match self.rpc(&tl::functions::messages::GetSavedGifs { hash }).await? {
            tl::enums::messages::SavedGifs::Gifs(g) => {
                let docs = g.gifs.into_iter().filter_map(|d| match d {
                    tl::enums::Document::Document(d) => Some(d),
                    tl::enums::Document::Empty(_) => None,
                });
                let list = self.remember_gifs(docs);
                self.gif_catalog().saved = (g.hash, list.clone());
                Ok(list)
            }
            tl::enums::messages::SavedGifs::NotModified => Ok(self.gif_catalog().saved.1.clone()),
        }
    }

    /// El bot `@gif`, resuelto una vez por sesión.
    async fn gif_bot(&self) -> BackendResult<tl::enums::InputUser> {
        let known = self.gif_catalog().bot;
        let id = match known {
            Some(id) => id,
            None => {
                let tl::enums::contacts::ResolvedPeer::Peer(r) =
                    self.rpc(&tl::functions::contacts::ResolveUsername { username: GIF_BOT.into(), referer: None }).await?;
                self.ingest(&r.users, &r.chats);
                let tl::enums::Peer::User(u) = r.peer else { return Err(BackendError::NotFound) };
                self.gif_catalog().bot = Some(u.user_id);
                u.user_id
            }
        };
        let c = self.cache();
        c.peers.input_user(UserId(id), c.me, &self.net.session)
    }

    /// Busca GIFs con el bot inline. Solo devuelve los resultados que traen documento.
    pub(crate) async fn search_gifs(&self, query: String, offset: String) -> BackendResult<GifPage> {
        self.ensure_logged_in()?;
        let bot = self.gif_bot().await?;
        let req = tl::functions::messages::GetInlineBotResults { bot, peer: tl::enums::InputPeer::PeerSelf, geo_point: None, query, offset };
        let tl::enums::messages::BotResults::Results(r) = self.rpc(&req).await?;
        self.ingest(&r.users, &[]);
        let mut gifs = Vec::with_capacity(r.results.len());
        for res in r.results {
            let tl::enums::BotInlineResult::BotInlineMediaResult(m) = res else {
                tracing::debug!("resultado de GIF sin documento (URL externa): se omite");
                continue;
            };
            let Some(tl::enums::Document::Document(d)) = m.document else { continue };
            let mut g = self.remember_gifs([d]).remove(0);
            if let Some(t) = m.title.filter(|t| !t.trim().is_empty()) {
                g.title = t;
            }
            g.inline = Some(InlineResult { query_id: r.query_id, result_id: m.id });
            gifs.push(g);
        }
        Ok(GifPage { gifs, next_offset: r.next_offset.filter(|o| !o.is_empty()) })
    }

    /// Referencia fresca de un GIF guardado (se vuelven a pedir los guardados sin `hash`).
    pub(crate) async fn refresh_gif_file(&self, old: &FileRef) -> BackendResult<FileRef> {
        let list = self.fetch_saved_gifs(0).await?;
        list.into_iter().find(|g| g.id == old.id).and_then(|g| g.file).ok_or(BackendError::NotFound)
    }

    /// Referencia fresca de un archivo sin mensaje (`MsgId(0)`): sticker de un pack o GIF guardado.
    pub(crate) async fn refresh_loose_file(&self, old: &FileRef) -> BackendResult<FileRef> {
        match self.refresh_sticker_file(old).await {
            Err(BackendError::NotFound) => self.refresh_gif_file(old).await,
            r => r,
        }
    }

    pub(crate) async fn send_gif(&self, peer: PeerRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.ensure_logged_in()?;
        if let Some(inline) = gif.inline.clone() {
            match self.send_inline_result(peer, inline, reply_to, random_id).await {
                // Consulta caducada o resultado inválido: se envía el documento si lo hay.
                Err(BackendError::InvalidInput(_)) if gif.file.is_some() => {
                    tracing::debug!("resultado del bot caducado: se envía el documento del GIF");
                }
                r => return r,
            }
        }
        for attempt in 0..2 {
            let doc = self.gif_catalog().docs.get(&gif.id).cloned().or_else(|| {
                gif.file.as_ref().map(|f| tl::types::InputDocument { id: f.id, access_hash: f.access_hash, file_reference: f.file_reference.clone() })
            });
            let doc = doc.ok_or(BackendError::NotFound)?;
            let media = tl::types::InputMediaDocument { spoiler: false, id: doc.clone().into(), video_cover: None, video_timestamp: None, ttl_seconds: None, query: None };
            match self.send_media_item(peer, media.into(), String::new(), reply_to, random_id).await {
                Err(BackendError::InvalidInput("file_reference")) if attempt == 0 => {
                    tracing::debug!(id = gif.id, "referencia del GIF caducada: se refrescan los guardados");
                    self.fetch_saved_gifs(0).await?;
                }
                r => return r,
            }
        }
        Err(BackendError::NotFound)
    }

    async fn send_inline_result(&self, peer: PeerRef, inline: InlineResult, reply: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        let req = tl::functions::messages::SendInlineBotResult {
            silent: false,
            background: false,
            clear_draft: false,
            hide_via: true,
            peer: self.input_peer(peer)?,
            reply_to: crate::backend::reply_to(reply),
            random_id: random_id.0,
            query_id: inline.query_id,
            id: inline.result_id,
            schedule_date: None,
            send_as: None,
            quick_reply_shortcut: None,
            allow_paid_stars: None,
        };
        self.send_and_confirm(peer, random_id, self.rpc(&req), move |id, date, m| {
            let mut msg = self.outgoing(peer, id, date, String::new(), vec![], reply);
            if let Some(m) = m {
                (msg.media, msg.sticker) = crate::convert::media_from_tl(&m);
            }
            msg
        })
        .await
    }
}
