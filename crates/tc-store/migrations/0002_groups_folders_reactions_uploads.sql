-- teleClon · caché local SQLite · migración 0002 (PRAGMA user_version = 2)
-- Las tablas de grupos, carpetas, reacciones y subidas ya existen desde la 0001; aquí se añade lo que
-- faltaba para que los repositorios y el CachedBackend puedan reconstruir las entidades del dominio sin red.
--   * dialogs.title        : título visible del chat (la UI lo pinta sin resolver usuario/grupo).
--   * messages.sticker_json: sticker adjunto serializado (no exige que el pack esté instalado en `stickers`).
--   * uploads.last_error   : motivo del último fallo, para mostrarlo al reintentar.
--   * media.duration_ms ya existía en la 0001 (solo faltaba escribirla desde el código).
-- Los chats secretos siguen sin pasar por `messages` ni por FTS.

ALTER TABLE dialogs  ADD COLUMN title TEXT NOT NULL DEFAULT '';
ALTER TABLE messages ADD COLUMN sticker_json TEXT;
ALTER TABLE uploads  ADD COLUMN last_error TEXT;

-- Búsqueda de grupos por @usuario y listado de carpetas de un chat.
CREATE INDEX groups_username ON groups(username) WHERE username IS NOT NULL;
-- Reacciones propias de un chat (marcado de "no leídas" y limpieza).
CREATE INDEX reactions_chosen ON reactions(chat_key, msg_id) WHERE chosen = 1;
-- Subidas pendientes por antigüedad (reanudar en orden).
CREATE INDEX uploads_pending ON uploads(created_at) WHERE state IN ('queued','uploading','paused');

PRAGMA user_version = 2;
