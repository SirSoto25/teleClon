//! Caché en disco de los bytes descargados (fotos de perfil y medios), con índice LRU en SQLite.
//!
//! * Archivos en `<cuenta>/media/` (directorio 0700, archivos 0600 en Unix), uno por clave; la clave
//!   sale del id del archivo y del tamaño/variante (`profile_photo_key`, `media_key`).
//! * Tope de bytes: al guardar se borran los de uso más antiguo hasta quedar por debajo. Un archivo
//!   mayor que la cuarta parte del tope no se guarda.
//! * **Nunca** se guarda nada de un chat secreto: las funciones de clave devuelven `None` para ellos
//!   y [`fetch_cached`] entonces va directo a la red.
//!
//! [`MediaCacheLayer`] añade esta caché a cualquier `ChatBackend` sin tocar el resto de operaciones (la
//! usa el backend de la red real, que no pasa por `CachedBackend`).

use crate::db::Store;
use crate::error::{Error, Result};
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};
use tc_core::*;

/// Tope por defecto de la caché de archivos (512 MiB).
pub const DEFAULT_MEDIA_CACHE_LIMIT: u64 = 512 * 1024 * 1024;

/// Nombre del subdirectorio de la caché dentro del de la cuenta.
pub const MEDIA_DIR: &str = "media";

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Clave de caché de una foto de perfil; `None` en chats secretos.
pub fn profile_photo_key(peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> Option<String> {
    if matches!(peer, PeerRef::Secret(_)) {
        return None;
    }
    let s = match size {
        PhotoSize::Small => "s",
        PhotoSize::Big => "b",
    };
    Some(format!("pp{}-{s}", photo.id))
}

/// Clave de caché de un medio; `None` en chats secretos.
pub fn media_key(peer: PeerRef, file: &FileRef, v: MediaVariant) -> Option<String> {
    if matches!(peer, PeerRef::Secret(_)) {
        return None;
    }
    let k = match file.kind {
        FileKind::Photo => "ph",
        FileKind::Document => "doc",
    };
    let v = match v {
        MediaVariant::Thumb => "t",
        MediaVariant::Full => "f",
    };
    Some(format!("{k}{}-{v}", file.id))
}

/// Solo `[A-Za-z0-9_-]`: la clave es también el nombre del archivo (nada de rutas).
fn check_key(key: &str) -> Result<()> {
    let ok = !key.is_empty() && key.len() <= 96 && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if ok { Ok(()) } else { Err(Error::Invalid(format!("clave de caché no válida: {key:?}"))) }
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io { path: path.into(), source }
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir).map_err(io(dir))?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(io(dir))
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(io(dir))
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path).map_err(io(path))?;
    f.write_all(bytes).map_err(io(path))
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    std::fs::write(path, bytes).map_err(io(path))
}

/// Caché de archivos de una cuenta. Síncrona (usar desde `spawn_blocking`); se comparte con `Arc`.
pub struct MediaCache {
    dir: PathBuf,
    limit: u64,
    index: Mutex<Store>,
}

impl std::fmt::Debug for MediaCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MediaCache").field("dir", &self.dir).field("limit", &self.limit).finish_non_exhaustive()
    }
}

impl MediaCache {
    /// Caché en `dir` (se crea con permisos 0700) con el índice en `index` y un tope de `limit` bytes.
    pub fn open(dir: impl Into<PathBuf>, index: Store, limit: u64) -> Result<Self> {
        let dir = dir.into();
        create_private_dir(&dir)?;
        // Restos de escrituras interrumpidas.
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().extension().is_some_and(|x| x == "part") {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let cache = Self { dir, limit, index: Mutex::new(index) };
        {
            let s = cache.store();
            cache.evict(&s, None)?;
        }
        Ok(cache)
    }

    /// Caché de una cuenta: `<root>/accounts/<id>/media/`, con el índice en la base de esa cuenta.
    pub fn open_account(root: impl AsRef<Path>, account_id: &str, limit: u64) -> Result<Self> {
        let db = crate::db::account_db_path(root.as_ref(), account_id)?;
        let dir = db.parent().map_or_else(|| PathBuf::from(MEDIA_DIR), |p| p.join(MEDIA_DIR));
        Self::open(dir, Store::open(&db)?, limit)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn limit(&self) -> u64 {
        self.limit
    }

    pub(crate) fn store(&self) -> MutexGuard<'_, Store> {
        self.index.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Bytes guardados, o `None` si no están (o el archivo desapareció o no cuadra con el índice).
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        check_key(key)?;
        let s = self.store();
        let Some(e) = s.cache_entry(key)? else { return Ok(None) };
        let path = self.dir.join(&e.file);
        match std::fs::read(&path) {
            Ok(b) if i64::try_from(b.len()).ok() == Some(e.size) => {
                s.cache_touch(key, now_ms())?;
                Ok(Some(b))
            }
            _ => {
                let _ = std::fs::remove_file(&path);
                s.cache_remove(key)?;
                Ok(None)
            }
        }
    }

    /// Guarda `bytes` con `key` y recorta la caché al tope. Devuelve `false` si no se guardó (vacío o
    /// demasiado grande para esta caché).
    pub fn put(&self, key: &str, bytes: &[u8]) -> Result<bool> {
        check_key(key)?;
        if bytes.is_empty() || bytes.len() as u64 > self.limit / 4 {
            return Ok(false);
        }
        let s = self.store();
        let part = self.dir.join(format!("{key}.part"));
        let path = self.dir.join(key);
        write_private(&part, bytes)?;
        std::fs::rename(&part, &path).map_err(io(&path))?;
        s.cache_put(&crate::files::CacheEntry { key: key.into(), file: key.into(), size: bytes.len() as i64, last_used_at: now_ms() })?;
        self.evict(&s, Some(key))?;
        Ok(true)
    }

    /// Bytes indexados ahora mismo.
    pub fn usage(&self) -> Result<u64> {
        Ok(u64::try_from(self.store().cache_total()?).unwrap_or(0))
    }

    /// Borra todo (archivos e índice).
    pub fn clear(&self) -> Result<()> {
        let s = self.store();
        for e in s.cache_all()? {
            let _ = std::fs::remove_file(self.dir.join(&e.file));
            s.cache_remove(&e.key)?;
        }
        Ok(())
    }

    /// Borra los de uso más antiguo (menos `keep`) hasta quedar dentro del tope.
    fn evict(&self, s: &Store, keep: Option<&str>) -> Result<()> {
        let limit = i64::try_from(self.limit).unwrap_or(i64::MAX);
        let mut total = s.cache_total()?;
        while total > limit {
            let mut progressed = false;
            for e in s.cache_oldest(32)? {
                if Some(e.key.as_str()) == keep {
                    continue;
                }
                let _ = std::fs::remove_file(self.dir.join(&e.file));
                s.cache_remove(&e.key)?;
                total -= e.size;
                progressed = true;
                if total <= limit {
                    break;
                }
            }
            if !progressed {
                break;
            }
        }
        Ok(())
    }
}

/// Sirve `key` desde la caché o, si no está, ejecuta `fetch` y guarda el resultado. Sin caché o sin
/// clave (chats secretos) va directo a `fetch`. Los fallos de la caché solo se registran.
pub async fn fetch_cached<F>(cache: Option<Arc<MediaCache>>, key: Option<String>, fetch: F) -> BackendResult<Vec<u8>>
where
    F: Future<Output = BackendResult<Vec<u8>>>,
{
    let (Some(cache), Some(key)) = (cache, key) else { return fetch.await };
    let (c, k) = (Arc::clone(&cache), key.clone());
    match tokio::task::spawn_blocking(move || c.get(&k)).await {
        Ok(Ok(Some(bytes))) => return Ok(bytes),
        Ok(Ok(None)) => {}
        Ok(Err(e)) => tracing::warn!(error = %e, "caché de archivos ilegible"),
        Err(_) => tracing::warn!("caché de archivos: tarea interrumpida"),
    }
    let bytes = fetch.await?;
    let copy = bytes.clone();
    match tokio::task::spawn_blocking(move || cache.put(&key, &copy)).await {
        Ok(Ok(_)) => {}
        Ok(Err(e)) => tracing::warn!(error = %e, "no se pudo guardar en la caché de archivos"),
        Err(_) => tracing::warn!("caché de archivos: tarea interrumpida"),
    }
    Ok(bytes)
}

/// Decorador que solo añade la caché de archivos a un backend (todo lo demás pasa tal cual).
pub struct MediaCacheLayer<B: ChatBackend> {
    inner: Arc<B>,
    cache: Arc<MediaCache>,
}

impl<B: ChatBackend> MediaCacheLayer<B> {
    pub fn new(inner: Arc<B>, cache: Arc<MediaCache>) -> Self {
        Self { inner, cache }
    }

    pub fn inner(&self) -> &Arc<B> {
        &self.inner
    }

    pub fn cache(&self) -> &Arc<MediaCache> {
        &self.cache
    }
}

#[async_trait]
impl<B: ChatBackend> ProfileApi for MediaCacheLayer<B> {
    async fn me(&self) -> BackendResult<User> {
        self.inner.me().await
    }
    async fn user(&self, id: UserId) -> BackendResult<User> {
        self.inner.user(id).await
    }
    async fn group(&self, id: GroupId) -> BackendResult<Group> {
        self.inner.group(id).await
    }
    async fn contacts(&self) -> BackendResult<Vec<User>> {
        self.inner.contacts().await
    }
    async fn set_blocked(&self, user: UserId, blocked: bool) -> BackendResult<()> {
        self.inner.set_blocked(user, blocked).await
    }
    async fn chat_info(&self, peer: PeerRef) -> BackendResult<tc_core::ChatInfo> {
        self.inner.chat_info(peer).await
    }
}

#[async_trait]
impl<B: ChatBackend> DialogsApi for MediaCacheLayer<B> {
    async fn dialogs(&self, page: DialogsPage) -> BackendResult<Vec<Dialog>> {
        self.inner.dialogs(page).await
    }
    async fn set_pinned(&self, peer: PeerRef, pinned: bool) -> BackendResult<()> {
        self.inner.set_pinned(peer, pinned).await
    }
    async fn set_archived(&self, peer: PeerRef, archived: bool) -> BackendResult<()> {
        self.inner.set_archived(peer, archived).await
    }
    async fn set_muted(&self, peer: PeerRef, muted: bool) -> BackendResult<()> {
        self.inner.set_muted(peer, muted).await
    }
    async fn save_draft(&self, peer: PeerRef, draft: Option<Draft>) -> BackendResult<()> {
        self.inner.save_draft(peer, draft).await
    }
    async fn mute_until(&self, peer: PeerRef, until: Option<tc_core::Timestamp>) -> BackendResult<()> {
        self.inner.mute_until(peer, until).await
    }
    async fn set_unread_mark(&self, peer: PeerRef, unread: bool) -> BackendResult<()> {
        self.inner.set_unread_mark(peer, unread).await
    }
    async fn mark_dialog_read(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.mark_dialog_read(peer).await
    }
    async fn clear_history(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.clear_history(peer).await
    }
    async fn delete_dialog(&self, peer: PeerRef, revoke: bool) -> BackendResult<()> {
        self.inner.delete_dialog(peer, revoke).await
    }
    async fn leave_chat(&self, peer: PeerRef) -> BackendResult<()> {
        self.inner.leave_chat(peer).await
    }
}

#[async_trait]
impl<B: ChatBackend> MessagesApi for MediaCacheLayer<B> {
    async fn history(&self, peer: PeerRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.inner.history(peer, anchor, limit).await
    }
    async fn send_text(&self, peer: PeerRef, msg: OutgoingText) -> BackendResult<Message> {
        self.inner.send_text(peer, msg).await
    }
    async fn edit_message(&self, peer: PeerRef, id: MsgId, text: String, entities: Vec<TextEntity>) -> BackendResult<Message> {
        self.inner.edit_message(peer, id, text, entities).await
    }
    async fn delete_messages(&self, peer: PeerRef, ids: Vec<MsgId>, revoke: bool) -> BackendResult<()> {
        self.inner.delete_messages(peer, ids, revoke).await
    }
    async fn forward_messages(&self, from: PeerRef, ids: Vec<MsgId>, to: Vec<PeerRef>, hide_sender: bool) -> BackendResult<()> {
        self.inner.forward_messages(from, ids, to, hide_sender).await
    }
    async fn send_reaction(&self, peer: PeerRef, id: MsgId, emoji: Option<String>) -> BackendResult<()> {
        self.inner.send_reaction(peer, id, emoji).await
    }
    async fn available_reactions(&self, peer: PeerRef) -> BackendResult<Vec<String>> {
        self.inner.available_reactions(peer).await
    }
    async fn mark_read(&self, peer: PeerRef, max_id: MsgId) -> BackendResult<()> {
        self.inner.mark_read(peer, max_id).await
    }
    async fn set_typing(&self, peer: PeerRef, typing: bool) -> BackendResult<()> {
        self.inner.set_typing(peer, typing).await
    }
    async fn pin_message(&self, peer: PeerRef, id: MsgId, pinned: bool) -> BackendResult<()> {
        self.inner.pin_message(peer, id, pinned).await
    }
}

#[async_trait]
impl<B: ChatBackend> MediaApi for MediaCacheLayer<B> {
    async fn send_media(&self, peer: PeerRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        self.inner.send_media(peer, media).await
    }
    async fn download_profile_photo(&self, peer: PeerRef, photo: ProfilePhoto, size: PhotoSize) -> BackendResult<Vec<u8>> {
        let key = profile_photo_key(peer, photo, size);
        fetch_cached(Some(Arc::clone(&self.cache)), key, self.inner.download_profile_photo(peer, photo, size)).await
    }
    async fn download_media(&self, peer: PeerRef, msg: MsgId, file: FileRef, variant: MediaVariant) -> BackendResult<Vec<u8>> {
        let key = media_key(peer, &file, variant);
        fetch_cached(Some(Arc::clone(&self.cache)), key, self.inner.download_media(peer, msg, file, variant)).await
    }
}

#[async_trait]
impl<B: ChatBackend> StickersApi for MediaCacheLayer<B> {
    async fn installed_sticker_sets(&self) -> BackendResult<Vec<StickerSet>> {
        self.inner.installed_sticker_sets().await
    }
    async fn send_sticker(&self, peer: PeerRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_sticker(peer, sticker, reply_to, random_id).await
    }
    async fn recent_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.recent_stickers().await
    }
    async fn faved_stickers(&self) -> BackendResult<Vec<Sticker>> {
        self.inner.faved_stickers().await
    }
    /// GIFs guardados: lo de la red se guarda; sin red, lo último guardado (ver `gifs.rs`).
    async fn saved_gifs(&self) -> BackendResult<Vec<tc_core::Gif>> {
        let net = self.inner.saved_gifs().await;
        crate::gifs::saved_gifs(self.cache.clone(), net).await
    }
    async fn search_gifs(&self, query: String, offset: String) -> BackendResult<tc_core::GifPage> {
        self.inner.search_gifs(query, offset).await
    }
    async fn send_gif(&self, peer: PeerRef, gif: tc_core::Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_gif(peer, gif, reply_to, random_id).await
    }
    /// El uso de emojis vive en la base de la cuenta (la red no lo guarda).
    async fn emoji_usage(&self) -> BackendResult<Vec<tc_core::EmojiUse>> {
        crate::gifs::emoji_usage(self.cache.clone()).await
    }
    async fn record_emoji_use(&self, emoji: String, at: Timestamp) -> BackendResult<()> {
        crate::gifs::record_emoji_use(self.cache.clone(), emoji, at).await
    }
}

#[async_trait]
impl<B: ChatBackend> SecretChatsApi for MediaCacheLayer<B> {
    async fn secret_chats(&self) -> BackendResult<Vec<SecretChat>> {
        self.inner.secret_chats().await
    }
    async fn request_secret_chat(&self, user: UserId) -> BackendResult<SecretChat> {
        self.inner.request_secret_chat(user).await
    }
    async fn accept_secret_chat(&self, id: SecretChatId) -> BackendResult<SecretChat> {
        self.inner.accept_secret_chat(id).await
    }
    async fn discard_secret_chat(&self, id: SecretChatId) -> BackendResult<()> {
        self.inner.discard_secret_chat(id).await
    }
    async fn set_secret_ttl(&self, id: SecretChatId, ttl_secs: Option<u32>) -> BackendResult<()> {
        self.inner.set_secret_ttl(id, ttl_secs).await
    }
    async fn secret_key_visual(&self, id: SecretChatId) -> BackendResult<KeyVisual> {
        self.inner.secret_key_visual(id).await
    }
}

/// Grupos con temas: pasan tal cual.
#[async_trait]
impl<B: ChatBackend> ForumApi for MediaCacheLayer<B> {
    async fn forum_topics(&self, group: GroupId, query: TopicsQuery) -> BackendResult<TopicsPage> {
        self.inner.forum_topics(group, query).await
    }
    async fn forum_topics_by_id(&self, group: GroupId, ids: Vec<MsgId>) -> BackendResult<Vec<ForumTopic>> {
        self.inner.forum_topics_by_id(group, ids).await
    }
    async fn topic_history(&self, thread: ThreadRef, anchor: Anchor, limit: usize) -> BackendResult<Vec<Message>> {
        self.inner.topic_history(thread, anchor, limit).await
    }
    async fn send_topic_text(&self, thread: ThreadRef, msg: OutgoingText) -> BackendResult<Message> {
        self.inner.send_topic_text(thread, msg).await
    }
    async fn send_topic_media(&self, thread: ThreadRef, media: OutgoingMedia) -> BackendResult<Vec<Message>> {
        self.inner.send_topic_media(thread, media).await
    }
    async fn send_topic_sticker(&self, thread: ThreadRef, sticker: StickerId, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_topic_sticker(thread, sticker, reply_to, random_id).await
    }
    async fn send_topic_gif(&self, thread: ThreadRef, gif: Gif, reply_to: Option<MsgId>, random_id: RandomId) -> BackendResult<Message> {
        self.inner.send_topic_gif(thread, gif, reply_to, random_id).await
    }
    async fn mark_topic_read(&self, thread: ThreadRef, max_id: MsgId) -> BackendResult<()> {
        self.inner.mark_topic_read(thread, max_id).await
    }
    async fn set_topic_typing(&self, thread: ThreadRef, typing: bool) -> BackendResult<()> {
        self.inner.set_topic_typing(thread, typing).await
    }
    async fn create_topic(&self, group: GroupId, title: String, icon_color: Option<u32>, random_id: RandomId) -> BackendResult<ForumTopic> {
        self.inner.create_topic(group, title, icon_color, random_id).await
    }
    async fn set_topic_pinned(&self, thread: ThreadRef, pinned: bool) -> BackendResult<()> {
        self.inner.set_topic_pinned(thread, pinned).await
    }
    async fn set_topic_closed(&self, thread: ThreadRef, closed: bool) -> BackendResult<()> {
        self.inner.set_topic_closed(thread, closed).await
    }
}

impl<B: ChatBackend> ChatBackend for MediaCacheLayer<B> {
    fn events(&self) -> BoxStream<'static, BackendEvent> {
        self.inner.events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) struct TmpDir(pub PathBuf);
    impl TmpDir {
        pub fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!("tc-media-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = std::fs::remove_dir_all(&p);
            Self(p)
        }
    }
    impl Drop for TmpDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn photo_ref(id: i64) -> FileRef {
        FileRef { kind: FileKind::Photo, id, access_hash: 1, file_reference: vec![], dc_id: 2, size_bytes: 3, thumb: None, full: None }
    }

    #[test]
    fn keys_depend_on_id_and_size_and_skip_secrets() {
        let p = ProfilePhoto { id: 42, dc_id: 2 };
        assert_eq!(profile_photo_key(PeerRef::User(UserId(1)), p, PhotoSize::Small).as_deref(), Some("pp42-s"));
        assert_eq!(profile_photo_key(PeerRef::Group(GroupId(1)), p, PhotoSize::Big).as_deref(), Some("pp42-b"));
        assert_eq!(profile_photo_key(PeerRef::Secret(SecretChatId(1)), p, PhotoSize::Small), None);
        let f = photo_ref(-7);
        assert_eq!(media_key(PeerRef::User(UserId(1)), &f, MediaVariant::Thumb).as_deref(), Some("ph-7-t"));
        assert_eq!(media_key(PeerRef::Secret(SecretChatId(1)), &f, MediaVariant::Full), None);
        assert!(check_key("../x").is_err() && check_key("a/b").is_err() && check_key("").is_err());
    }

    #[test]
    fn put_get_lru_eviction_and_private_files() {
        let tmp = TmpDir::new("lru");
        let dir = tmp.0.join("media");
        let c = MediaCache::open(&dir, Store::open_in_memory().unwrap(), 100).unwrap();
        assert!(c.put("a", &[1; 20]).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(3));
        assert!(c.put("b", &[2; 20]).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(3));
        // Usar «a» la hace más reciente que «b».
        assert_eq!(c.get("a").unwrap(), Some(vec![1; 20]));
        std::thread::sleep(std::time::Duration::from_millis(3));
        assert!(c.put("c", &[3; 20]).unwrap());
        assert!(!c.put("enorme", &[0; 26]).unwrap(), "más de un cuarto del tope no se guarda");
        assert_eq!(c.usage().unwrap(), 60);
        std::thread::sleep(std::time::Duration::from_millis(3));
        assert!(c.put("d", &[4; 25]).unwrap());
        assert!(c.put("e", &[5; 25]).unwrap());
        // Tope 100: sale «b», la de uso más antiguo («a» se usó después).
        assert_eq!(c.get("b").unwrap(), None);
        assert!(c.usage().unwrap() <= 100);
        assert_eq!(c.get("e").unwrap(), Some(vec![5; 25]));
        assert!(!dir.join("b").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&dir).unwrap().permissions().mode() & 0o777, 0o700);
            assert_eq!(std::fs::metadata(dir.join("e")).unwrap().permissions().mode() & 0o777, 0o600);
        }
        // Archivo borrado por fuera: la entrada se olvida.
        std::fs::remove_file(dir.join("e")).unwrap();
        assert_eq!(c.get("e").unwrap(), None);
        c.clear().unwrap();
        assert_eq!(c.usage().unwrap(), 0);
    }

    #[test]
    fn account_cache_lives_next_to_the_database_and_survives_reopen() {
        let tmp = TmpDir::new("acct");
        {
            let c = MediaCache::open_account(&tmp.0, "u1", DEFAULT_MEDIA_CACHE_LIMIT).unwrap();
            assert_eq!(c.dir(), tmp.0.join("accounts/u1/media"));
            c.put("pp1-s", b"jpeg").unwrap();
        }
        let c = MediaCache::open_account(&tmp.0, "u1", DEFAULT_MEDIA_CACHE_LIMIT).unwrap();
        assert_eq!(c.get("pp1-s").unwrap().as_deref(), Some(&b"jpeg"[..]));
    }

    #[tokio::test]
    async fn fetch_cached_goes_to_the_network_once_and_never_for_secrets() {
        let tmp = TmpDir::new("fetch");
        let c = Arc::new(MediaCache::open(tmp.0.join("m"), Store::open_in_memory().unwrap(), 1 << 20).unwrap());
        let calls = std::sync::atomic::AtomicUsize::new(0);
        let net = || async {
            calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(vec![9u8; 10])
        };
        let p = ProfilePhoto { id: 5, dc_id: 1 };
        for _ in 0..3 {
            let k = profile_photo_key(PeerRef::User(UserId(1)), p, PhotoSize::Small);
            assert_eq!(fetch_cached(Some(c.clone()), k, net()).await.unwrap(), vec![9; 10]);
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        for _ in 0..2 {
            let k = profile_photo_key(PeerRef::Secret(SecretChatId(3)), p, PhotoSize::Big);
            fetch_cached(Some(c.clone()), k, net()).await.unwrap();
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert_eq!(c.usage().unwrap(), 10, "nada del chat secreto en disco");
        // Un error de red no se guarda.
        let k = media_key(PeerRef::User(UserId(1)), &photo_ref(8), MediaVariant::Thumb);
        assert_eq!(fetch_cached(Some(c.clone()), k, async { Err(BackendError::Network) }).await, Err(BackendError::Network));
        assert_eq!(c.usage().unwrap(), 10);
    }
}
