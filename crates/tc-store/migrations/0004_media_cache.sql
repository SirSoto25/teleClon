-- Parlo · caché local SQLite · migración 0004 (PRAGMA user_version = 4)
-- Descarga de fotos de perfil y de medios (ver crates/tc-store/src/files.rs y media_cache.rs).
--   * peer_photos : foto de perfil vigente de cada usuario o grupo/canal (id de foto + DC). Un diálogo usa
--                   la de su interlocutor. Nunca se guarda nada de chats secretos (ni su foto).
--   * media_refs  : referencia de descarga de cada medio de un mensaje NO secreto (las columnas
--                   access_hash/file_reference/dc_id de `media` de la 0001 siguen sin usarse: la referencia
--                   necesita además el tipo de archivo y las variantes, y va aparte para no tocar `media`).
--   * file_cache  : índice LRU de los archivos descargados en `<cuenta>/media/` (0600). `key` es la clave
--                   derivada de id + tamaño/variante; `last_used_at` en milisegundos.

CREATE TABLE peer_photos (
    peer_kind TEXT    NOT NULL CHECK (peer_kind IN ('user','group')),
    peer_id   INTEGER NOT NULL,
    photo_id  INTEGER NOT NULL,
    dc_id     INTEGER NOT NULL,
    PRIMARY KEY (peer_kind, peer_id)
) WITHOUT ROWID;

CREATE TABLE media_refs (
    media_id       INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
    kind           TEXT    NOT NULL CHECK (kind IN ('photo','document')),
    access_hash    INTEGER NOT NULL,
    file_reference BLOB    NOT NULL,
    dc_id          INTEGER NOT NULL,
    size           INTEGER NOT NULL CHECK (size >= 0),
    thumb          TEXT,
    full           TEXT
);

CREATE TABLE file_cache (
    key          TEXT    PRIMARY KEY,
    file         TEXT    NOT NULL,
    size         INTEGER NOT NULL CHECK (size >= 0),
    last_used_at INTEGER NOT NULL
);
CREATE INDEX file_cache_lru ON file_cache(last_used_at);
