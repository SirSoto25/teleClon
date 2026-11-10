//! Stickers de la cuenta: packs instalados (`messages.getAllStickers` + `messages.getStickerSet`),
//! recientes (`messages.getRecentStickers`) y favoritos (`messages.getFavedStickers`), con los `hash`
//! de la API para no volver a bajar lo que no ha cambiado (caché en memoria de la sesión).
//!
//! Cada documento visto se recuerda (para `send_sticker`) junto con su pack, de modo que una
//! `file_reference` caducada se refresca volviendo a pedir el pack (`refresh_sticker_file`).
use std::collections::HashMap;

use grammers_tl_types as tl;
use tc_core::*;

use crate::convert::{document_file_ref, sticker_from_document};

/// Tope de packs que se piden en una carga (cada pack es una llamada; más provoca FLOOD_WAIT).
pub(crate) const MAX_SETS: usize = 50;

/// Pack ya bajado: su `hash` (cambia cuando cambia el pack) y el modelo.
#[derive(Clone)]
struct CachedSet {
    hash: i32,
    set: StickerSet,
}

/// Lo que se sabe de los stickers de la cuenta en esta sesión.
#[derive(Default)]
pub(crate) struct StickerCatalog {
    /// `hash` de `messages.allStickers` de la última carga completa (0 = no hay o fue parcial).
    all_hash: i64,
    /// Última lista de packs, en el orden de la cuenta.
    sets: Vec<StickerSet>,
    by_set: HashMap<i64, CachedSet>,
    recent: (i64, Vec<Sticker>),
    faved: (i64, Vec<Sticker>),
    /// Documento → pack `(id, access_hash)`, para refrescar referencias de stickers sin mensaje.
    doc_set: HashMap<i64, (i64, i64)>,
}

/// Pack del atributo de sticker de un documento (solo si viene por id; los de nombre corto no
/// sirven para refrescar sin otra llamada).
fn set_of(d: &tl::types::Document) -> Option<(i64, i64)> {
    d.attributes.iter().find_map(|a| match a {
        tl::enums::DocumentAttribute::Sticker(s) => match &s.stickerset {
            tl::enums::InputStickerSet::Id(x) => Some((x.id, x.access_hash)),
            _ => None,
        },
        _ => None,
    })
}

impl crate::Inner {
    fn catalog(&self) -> std::sync::MutexGuard<'_, StickerCatalog> {
        self.stickers.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Traduce los documentos de stickers y los recuerda para enviarlos y refrescarlos.
    /// `set` fuerza el pack (los documentos de un `stickerSet` lo traen implícito).
    fn remember_docs(&self, docs: Vec<tl::enums::Document>, set: Option<(i64, i64)>) -> Vec<Sticker> {
        let mut out = Vec::with_capacity(docs.len());
        let mut links = Vec::new();
        {
            let mut c = self.cache();
            for d in docs {
                let tl::enums::Document::Document(d) = d else { continue };
                let Some(mut st) = sticker_from_document(&d) else { continue };
                let link = set.or_else(|| set_of(&d));
                if let Some((id, _)) = link {
                    st.set_id = StickerSetId(id);
                }
                if let Some(l) = link {
                    links.push((d.id, l));
                }
                c.stickers.insert(d.id, tl::types::InputDocument { id: d.id, access_hash: d.access_hash, file_reference: d.file_reference.clone() });
                out.push(st);
            }
        }
        self.catalog().doc_set.extend(links);
        out
    }

    /// Un pack completo por id.
    async fn fetch_set(&self, id: i64, access_hash: i64) -> BackendResult<Option<Vec<tl::enums::Document>>> {
        let req = tl::functions::messages::GetStickerSet { stickerset: tl::types::InputStickerSetId { id, access_hash }.into(), hash: 0 };
        Ok(match self.rpc(&req).await? {
            tl::enums::messages::StickerSet::Set(full) => Some(full.documents),
            tl::enums::messages::StickerSet::NotModified => None,
        })
    }

    pub(crate) async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        self.ensure_logged_in()?;
        let hash = self.catalog().all_hash;
        let (new_hash, list) = match self.rpc(&tl::functions::messages::GetAllStickers { hash }).await? {
            tl::enums::messages::AllStickers::Stickers(s) => (s.hash, s.sets),
            tl::enums::messages::AllStickers::NotModified => return Ok(self.catalog().sets.clone()),
        };
        let mut out = Vec::new();
        let mut complete = list.len() <= MAX_SETS;
        for s in list.into_iter().take(MAX_SETS) {
            let tl::enums::StickerSet::Set(s) = s;
            // Sin cambios desde la última vez: ni se pide.
            let cached = self.catalog().by_set.get(&s.id).filter(|c| c.hash == s.hash).map(|c| c.set.clone());
            if let Some(set) = cached {
                out.push(set);
                continue;
            }
            match self.fetch_set(s.id, s.access_hash).await {
                Ok(Some(docs)) => {
                    let stickers = self.remember_docs(docs, Some((s.id, s.access_hash)));
                    let set = StickerSet { id: StickerSetId(s.id), short_name: s.short_name, title: s.title, stickers };
                    self.catalog().by_set.insert(s.id, CachedSet { hash: s.hash, set: set.clone() });
                    out.push(set);
                }
                Ok(None) => complete = false,
                // Con FLOOD_WAIT se devuelve lo que haya; la próxima carga sigue (el hash no se guarda).
                Err(BackendError::FloodWait(_)) => {
                    complete = false;
                    break;
                }
                // Un pack roto o retirado no impide ver los demás.
                Err(e) => {
                    tracing::warn!(error = %e, "no se pudo cargar un pack de stickers");
                    complete = false;
                }
            }
        }
        let mut cat = self.catalog();
        cat.all_hash = if complete { new_hash } else { 0 };
        cat.sets = out.clone();
        Ok(out)
    }

    pub(crate) async fn recent_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.ensure_logged_in()?;
        let hash = self.catalog().recent.0;
        match self.rpc(&tl::functions::messages::GetRecentStickers { attached: false, hash }).await? {
            tl::enums::messages::RecentStickers::Stickers(r) => {
                let list = self.remember_docs(r.stickers, None);
                self.catalog().recent = (r.hash, list.clone());
                Ok(list)
            }
            tl::enums::messages::RecentStickers::NotModified => Ok(self.catalog().recent.1.clone()),
        }
    }

    pub(crate) async fn faved_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.ensure_logged_in()?;
        let hash = self.catalog().faved.0;
        match self.rpc(&tl::functions::messages::GetFavedStickers { hash }).await? {
            tl::enums::messages::FavedStickers::Stickers(r) => {
                let list = self.remember_docs(r.stickers, None);
                self.catalog().faved = (r.hash, list.clone());
                Ok(list)
            }
            tl::enums::messages::FavedStickers::NotModified => Ok(self.catalog().faved.1.clone()),
        }
    }

    /// Referencia fresca de un sticker sin mensaje: se vuelve a pedir su pack.
    pub(crate) async fn refresh_sticker_file(&self, old: &FileRef) -> BackendResult<FileRef> {
        let (set_id, access_hash) = self.catalog().doc_set.get(&old.id).copied().ok_or(BackendError::NotFound)?;
        let docs = self.fetch_set(set_id, access_hash).await?.ok_or(BackendError::NotFound)?;
        let fresh = docs.iter().find_map(|d| match d {
            tl::enums::Document::Document(d) if d.id == old.id => Some(document_file_ref(d)),
            _ => None,
        });
        // Todo el pack lleva referencias nuevas: se recuerdan (envíos y otras descargas).
        self.remember_docs(docs, Some((set_id, access_hash)));
        if let Some(c) = self.catalog().by_set.get_mut(&set_id) {
            c.hash = 0; // el modelo guardado tiene referencias viejas: que la próxima carga lo rehaga
        }
        fresh.ok_or(BackendError::NotFound)
    }

    pub(crate) async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        for attempt in 0..2 {
            let doc = self.cache().stickers.get(&sticker.0).cloned().ok_or(BackendError::NotFound)?;
            let media = tl::types::InputMediaDocument { spoiler: false, id: doc.clone().into(), video_cover: None, video_timestamp: None, ttl_seconds: None, query: None };
            match self.send_media_item(peer, media.into(), String::new(), reply_to, random_id).await {
                Err(BackendError::InvalidInput("file_reference")) if attempt == 0 => {
                    tracing::debug!(id = sticker.0, "referencia del sticker caducada: se refresca el pack");
                    let old = FileRef {
                        kind: FileKind::Document,
                        id: doc.id,
                        access_hash: doc.access_hash,
                        file_reference: doc.file_reference,
                        dc_id: 0,
                        size_bytes: 0,
                        thumb: None,
                        full: None,
                    };
                    self.refresh_sticker_file(&old).await?;
                }
                r => return r,
            }
        }
        Err(BackendError::NotFound)
    }
}
