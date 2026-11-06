-- teleClon · caché local SQLite · migración 0001 (PRAGMA user_version = 1)
-- Un archivo por cuenta: $XDG_DATA_HOME/<app>/accounts/<account_id>/cache.sqlite3 (permisos 0600).
-- Convenciones (ver docs/architecture.md §7):
--   * Es una CACHÉ de un cliente: los ids remotos son los de la red (enteros de 64 bits), no UUID.
--     Los chats usan una clave local sustituta `chat_key` porque usuarios, grupos, canales y chats
--     secretos viven en espacios de ids distintos.
--   * Tiempos: INTEGER segundos Unix en UTC (SQLite no tiene timestamptz). Se formatean a hora local en la UI.
--   * Booleanos: INTEGER 0/1 con CHECK.
--   * Sin "propietario" por fila: la base entera pertenece a una cuenta. La autorización real la hace el
--     servidor; localmente el control de acceso es el permiso 0600 del archivo + keyring para secretos.
--   * Nunca se guardan claves en esta base: solo referencias a entradas del keyring (`key_ref`).
-- PRAGMAs que aplica el código al abrir (no forman parte de la migración):
--   journal_mode=WAL; synchronous=NORMAL; foreign_keys=ON; secure_delete=ON; busy_timeout=5000.

CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
) WITHOUT ROWID;

-- Usuarios conocidos (contactos, remitentes, miembros). is_contact/saved_name cubren la entidad Contact.
CREATE TABLE users (
    id             INTEGER PRIMARY KEY,               -- id de usuario de la red
    access_hash    INTEGER,
    first_name     TEXT NOT NULL DEFAULT '',
    last_name      TEXT NOT NULL DEFAULT '',
    username       TEXT,
    phone          TEXT,
    bio            TEXT,
    photo_media_id INTEGER REFERENCES media(id) ON DELETE SET NULL,
    status         TEXT NOT NULL DEFAULT 'unknown'
                   CHECK (status IN ('unknown','online','offline','recently','last_week','last_month')),
    last_seen_at   INTEGER,
    is_bot         INTEGER NOT NULL DEFAULT 0 CHECK (is_bot IN (0,1)),
    is_self        INTEGER NOT NULL DEFAULT 0 CHECK (is_self IN (0,1)),
    is_contact     INTEGER NOT NULL DEFAULT 0 CHECK (is_contact IN (0,1)),
    is_mutual      INTEGER NOT NULL DEFAULT 0 CHECK (is_mutual IN (0,1)),
    is_blocked     INTEGER NOT NULL DEFAULT 0 CHECK (is_blocked IN (0,1)),
    saved_name     TEXT,
    updated_at     INTEGER NOT NULL
);
CREATE INDEX users_username ON users(username) WHERE username IS NOT NULL;
CREATE INDEX users_contacts ON users(is_contact, first_name, last_name) WHERE is_contact = 1;
CREATE INDEX users_photo ON users(photo_media_id);

-- Grupos, supergrupos y canales.
CREATE TABLE groups (
    id                 INTEGER PRIMARY KEY,           -- id de chat/canal de la red (sin marcar)
    kind               TEXT NOT NULL CHECK (kind IN ('group','supergroup','channel')),
    access_hash        INTEGER,
    title              TEXT NOT NULL,
    about              TEXT,
    username           TEXT,
    photo_media_id     INTEGER REFERENCES media(id) ON DELETE SET NULL,
    participants_count INTEGER,
    my_role            TEXT NOT NULL DEFAULT 'member'
                       CHECK (my_role IN ('creator','admin','member','left','banned')),
    can_send           INTEGER NOT NULL DEFAULT 1 CHECK (can_send IN (0,1)),
    created_at         INTEGER,
    updated_at         INTEGER NOT NULL
);
CREATE INDEX groups_photo ON groups(photo_media_id);

-- Chats secretos E2E (ADR-0004). Ligados a este dispositivo. La clave de 256 bytes vive en el keyring
-- (entrada `key_ref`); aquí solo el estado del protocolo. Nunca se sincroniza con otro dispositivo.
CREATE TABLE secret_chats (
    id              INTEGER PRIMARY KEY,              -- id de chat cifrado de la red (32 bits)
    access_hash     INTEGER NOT NULL,
    peer_user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    state           TEXT NOT NULL CHECK (state IN ('requested','waiting','accepted','discarded')),
    is_originator   INTEGER NOT NULL CHECK (is_originator IN (0,1)),
    layer           INTEGER NOT NULL DEFAULT 8,       -- capa negociada (notifyLayer)
    in_seq_no       INTEGER NOT NULL DEFAULT 0,
    out_seq_no      INTEGER NOT NULL DEFAULT 0,
    ttl_seconds     INTEGER NOT NULL DEFAULT 0 CHECK (ttl_seconds >= 0),
    key_ref         TEXT,                             -- nombre de la entrada del keyring; NULL antes de aceptar
    key_fingerprint INTEGER,                          -- 64 bits bajos de SHA1(clave): público, para comprobar
    key_used_count  INTEGER NOT NULL DEFAULT 0,       -- mensajes con la clave actual (re-keying cada 100)
    key_created_at  INTEGER,                          -- re-keying como mucho cada semana
    rekey_state     TEXT NOT NULL DEFAULT 'idle'
                    CHECK (rekey_state IN ('idle','requested','accepted','committing')),
    rekey_exchange_id INTEGER,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX secret_chats_peer ON secret_chats(peer_user_id);
CREATE INDEX secret_chats_state ON secret_chats(state);

-- Diálogos = filas de la lista de chats. `chat_key` es la clave local que usa todo lo demás.
CREATE TABLE dialogs (
    chat_key            INTEGER PRIMARY KEY,          -- clave local sustituta (rowid)
    peer_kind           TEXT NOT NULL CHECK (peer_kind IN ('user','group','secret')),
    peer_id             INTEGER NOT NULL,             -- users.id | groups.id | secret_chats.id según peer_kind
    top_message_id      INTEGER,
    last_message_at     INTEGER,                      -- orden por actividad
    unread_count        INTEGER NOT NULL DEFAULT 0 CHECK (unread_count >= 0),
    unread_mentions     INTEGER NOT NULL DEFAULT 0 CHECK (unread_mentions >= 0),
    unread_reactions    INTEGER NOT NULL DEFAULT 0 CHECK (unread_reactions >= 0),
    read_inbox_max_id   INTEGER NOT NULL DEFAULT 0,
    read_outbox_max_id  INTEGER NOT NULL DEFAULT 0,
    pinned_order        INTEGER,                      -- NULL = no fijado
    archived            INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
    mute_until          INTEGER NOT NULL DEFAULT 0,   -- 0 = no silenciado; i32::MAX = siempre
    draft_text          TEXT,
    draft_entities      TEXT,                         -- JSON de entidades
    draft_reply_to      INTEGER,
    draft_at            INTEGER,
    pinned_message_id   INTEGER,
    history_complete    INTEGER NOT NULL DEFAULT 0 CHECK (history_complete IN (0,1)), -- llegamos al primer mensaje
    updated_at          INTEGER NOT NULL,
    UNIQUE (peer_kind, peer_id)
);
-- Lista de chats: fijados primero, luego por actividad. Se filtra por archivados.
CREATE INDEX dialogs_list ON dialogs(archived, pinned_order, last_message_at DESC);
CREATE INDEX dialogs_unread ON dialogs(unread_count) WHERE unread_count > 0;

-- Medios y archivos (fotos, vídeos, documentos, voz, stickers, miniaturas de avatar).
CREATE TABLE media (
    id               INTEGER PRIMARY KEY,             -- id de documento/foto de la red (o local negativo para subidas)
    access_hash      INTEGER,
    file_reference   BLOB,
    dc_id            INTEGER,
    kind             TEXT NOT NULL
                     CHECK (kind IN ('photo','video','document','voice','audio','sticker','gif','avatar','geo')),
    mime             TEXT,
    size             INTEGER CHECK (size IS NULL OR size >= 0),
    width            INTEGER,
    height           INTEGER,
    duration_ms      INTEGER,
    file_name        TEXT,
    thumb_path       TEXT,                            -- miniatura en la caché de disco
    local_path       TEXT,                            -- archivo completo descargado
    download_state   TEXT NOT NULL DEFAULT 'none'
                     CHECK (download_state IN ('none','partial','done','failed')),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    is_secret        INTEGER NOT NULL DEFAULT 0 CHECK (is_secret IN (0,1)), -- archivo cifrado E2E: en disco siempre cifrado
    secret_key_ref   TEXT,                            -- clave/iv del archivo E2E en el keyring
    last_used_at     INTEGER,                         -- para el recolector de caché (LRU)
    updated_at       INTEGER NOT NULL
);
CREATE INDEX media_lru ON media(download_state, last_used_at);

-- Mensajes de chats normales (no secretos). rowid explícito para FTS5 de contenido externo.
CREATE TABLE messages (
    rowid           INTEGER PRIMARY KEY,
    chat_key        INTEGER NOT NULL REFERENCES dialogs(chat_key) ON DELETE CASCADE,
    msg_id          INTEGER NOT NULL,                 -- id de la red; negativo = pendiente local
    random_id       INTEGER,                          -- idempotencia de envíos (único por cuenta)
    sender_id       INTEGER,                          -- users.id o groups.id (canal que publica)
    date            INTEGER NOT NULL,
    edit_date       INTEGER,
    is_out          INTEGER NOT NULL CHECK (is_out IN (0,1)),
    kind            TEXT NOT NULL CHECK (kind IN ('text','media','sticker','service')),
    text            TEXT NOT NULL DEFAULT '',
    entities        TEXT,                             -- JSON [{type, offset, length, url?}] (offsets UTF-16)
    reply_to_id     INTEGER,
    reply_quote     TEXT,
    fwd_from_id     INTEGER,
    fwd_from_name   TEXT,
    fwd_date        INTEGER,
    media_id        INTEGER REFERENCES media(id) ON DELETE SET NULL,
    sticker_id      INTEGER REFERENCES stickers(id) ON DELETE SET NULL,
    grouped_id      INTEGER,                          -- álbum
    service_action  TEXT,                             -- JSON para mensajes de servicio
    is_pinned       INTEGER NOT NULL DEFAULT 0 CHECK (is_pinned IN (0,1)),
    views           INTEGER,
    send_state      TEXT NOT NULL DEFAULT 'sent'
                    CHECK (send_state IN ('pending','sent','failed')),
    UNIQUE (chat_key, msg_id)
);
-- Paginación del historial: la clave única (chat_key, msg_id) ya sirve "WHERE chat_key=? AND msg_id<? ORDER BY msg_id DESC LIMIT ?".
CREATE UNIQUE INDEX messages_random ON messages(random_id) WHERE random_id IS NOT NULL;
CREATE INDEX messages_date ON messages(chat_key, date);          -- salto por fecha, separadores
CREATE INDEX messages_album ON messages(chat_key, grouped_id) WHERE grouped_id IS NOT NULL;
CREATE INDEX messages_media ON messages(media_id) WHERE media_id IS NOT NULL;
CREATE INDEX messages_sticker ON messages(sticker_id) WHERE sticker_id IS NOT NULL;
CREATE INDEX messages_pending ON messages(send_state) WHERE send_state <> 'sent';
CREATE INDEX messages_sender ON messages(sender_id);

-- Búsqueda local de texto (complementa la búsqueda del servidor). Los chats secretos NO se indexan.
CREATE VIRTUAL TABLE messages_fts USING fts5(
    text, content='messages', content_rowid='rowid', tokenize='unicode61 remove_diacritics 2'
);
CREATE TRIGGER messages_fts_ai AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, text) VALUES (new.rowid, new.text);
END;
CREATE TRIGGER messages_fts_ad AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
END;
CREATE TRIGGER messages_fts_au AFTER UPDATE OF text ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, text) VALUES ('delete', old.rowid, old.text);
    INSERT INTO messages_fts(rowid, text) VALUES (new.rowid, new.text);
END;

-- Reacciones agregadas por mensaje (la red da recuentos, no la lista completa de usuarios).
CREATE TABLE reactions (
    chat_key  INTEGER NOT NULL,
    msg_id    INTEGER NOT NULL,
    emoji     TEXT NOT NULL,
    count     INTEGER NOT NULL CHECK (count >= 0),
    chosen    INTEGER NOT NULL DEFAULT 0 CHECK (chosen IN (0,1)), -- la mía
    PRIMARY KEY (chat_key, msg_id, emoji),
    FOREIGN KEY (chat_key, msg_id) REFERENCES messages(chat_key, msg_id) ON DELETE CASCADE ON UPDATE CASCADE
) WITHOUT ROWID;

-- Mensajes de chats secretos: cuerpo cifrado en reposo (AES-256-GCM con clave local del keyring),
-- fuera de FTS, con borrado programado por TTL.
CREATE TABLE secret_messages (
    rowid          INTEGER PRIMARY KEY,
    secret_chat_id INTEGER NOT NULL REFERENCES secret_chats(id) ON DELETE CASCADE,
    random_id      INTEGER NOT NULL,                  -- id del mensaje en el protocolo E2E
    is_out         INTEGER NOT NULL CHECK (is_out IN (0,1)),
    date           INTEGER NOT NULL,
    seq_in         INTEGER,
    seq_out        INTEGER,
    body_nonce     BLOB NOT NULL,                     -- 12 bytes
    body_enc       BLOB NOT NULL,                     -- DecryptedMessage serializado y re-cifrado en reposo
    media_id       INTEGER REFERENCES media(id) ON DELETE SET NULL,
    ttl_seconds    INTEGER NOT NULL DEFAULT 0 CHECK (ttl_seconds >= 0),
    read_at        INTEGER,                           -- el TTL empieza a contar al leerse
    destroy_at     INTEGER,                           -- read_at + ttl_seconds
    send_state     TEXT NOT NULL DEFAULT 'sent' CHECK (send_state IN ('pending','sent','failed')),
    UNIQUE (secret_chat_id, random_id)
);
CREATE INDEX secret_messages_history ON secret_messages(secret_chat_id, date);
CREATE INDEX secret_messages_destroy ON secret_messages(destroy_at) WHERE destroy_at IS NOT NULL;
CREATE INDEX secret_messages_media ON secret_messages(media_id) WHERE media_id IS NOT NULL;

-- Packs de stickers (los sirve la red; el pack propio del mock/marca también entra aquí).
CREATE TABLE sticker_sets (
    id              INTEGER PRIMARY KEY,
    access_hash     INTEGER,
    short_name      TEXT NOT NULL UNIQUE,
    title           TEXT NOT NULL,
    format          TEXT NOT NULL CHECK (format IN ('static','animated','video','mixed')),
    is_emoji        INTEGER NOT NULL DEFAULT 0 CHECK (is_emoji IN (0,1)),
    sticker_count   INTEGER NOT NULL DEFAULT 0,
    thumb_media_id  INTEGER REFERENCES media(id) ON DELETE SET NULL,
    installed       INTEGER NOT NULL DEFAULT 0 CHECK (installed IN (0,1)),
    installed_order INTEGER,
    archived        INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0,1)),
    remote_hash     INTEGER,                          -- para pedir solo cambios
    is_local        INTEGER NOT NULL DEFAULT 0 CHECK (is_local IN (0,1)), -- pack original empaquetado
    updated_at      INTEGER NOT NULL
);
CREATE INDEX sticker_sets_installed ON sticker_sets(installed, installed_order) WHERE installed = 1;
CREATE INDEX sticker_sets_thumb ON sticker_sets(thumb_media_id);

CREATE TABLE stickers (
    id        INTEGER PRIMARY KEY,                    -- id de documento
    set_id    INTEGER NOT NULL REFERENCES sticker_sets(id) ON DELETE CASCADE,
    media_id  INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
    emoji     TEXT NOT NULL DEFAULT '',
    format    TEXT NOT NULL CHECK (format IN ('webp','tgs','webm')),
    position  INTEGER NOT NULL
);
CREATE INDEX stickers_set ON stickers(set_id, position);
CREATE INDEX stickers_emoji ON stickers(emoji);
CREATE INDEX stickers_media ON stickers(media_id);

CREATE TABLE sticker_recent (
    sticker_id INTEGER PRIMARY KEY REFERENCES stickers(id) ON DELETE CASCADE,
    used_at    INTEGER NOT NULL
);
CREATE INDEX sticker_recent_used ON sticker_recent(used_at DESC);

CREATE TABLE sticker_faves (
    sticker_id INTEGER PRIMARY KEY REFERENCES stickers(id) ON DELETE CASCADE,
    added_at   INTEGER NOT NULL
);
CREATE INDEX sticker_faves_added ON sticker_faves(added_at DESC);

-- Carpetas (filtros de chats).
CREATE TABLE folders (
    id               INTEGER PRIMARY KEY,             -- id de filtro de la red
    title            TEXT NOT NULL,
    emoticon         TEXT,
    sort_order       INTEGER NOT NULL,
    inc_contacts     INTEGER NOT NULL DEFAULT 0 CHECK (inc_contacts IN (0,1)),
    inc_non_contacts INTEGER NOT NULL DEFAULT 0 CHECK (inc_non_contacts IN (0,1)),
    inc_groups       INTEGER NOT NULL DEFAULT 0 CHECK (inc_groups IN (0,1)),
    inc_channels     INTEGER NOT NULL DEFAULT 0 CHECK (inc_channels IN (0,1)),
    inc_bots         INTEGER NOT NULL DEFAULT 0 CHECK (inc_bots IN (0,1)),
    exc_muted        INTEGER NOT NULL DEFAULT 0 CHECK (exc_muted IN (0,1)),
    exc_read         INTEGER NOT NULL DEFAULT 0 CHECK (exc_read IN (0,1)),
    exc_archived     INTEGER NOT NULL DEFAULT 0 CHECK (exc_archived IN (0,1)),
    updated_at       INTEGER NOT NULL
);
CREATE INDEX folders_order ON folders(sort_order);

CREATE TABLE folder_peers (
    folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    chat_key  INTEGER NOT NULL REFERENCES dialogs(chat_key) ON DELETE CASCADE,
    mode      TEXT NOT NULL CHECK (mode IN ('include','exclude','pinned')),
    position  INTEGER,                                -- orden de los fijados en la carpeta
    PRIMARY KEY (folder_id, chat_key, mode)
) WITHOUT ROWID;
CREATE INDEX folder_peers_chat ON folder_peers(chat_key);

-- Ajustes de notificación por ámbito (por defecto de privados/grupos/canales) o por chat.
CREATE TABLE notify_settings (
    scope         TEXT NOT NULL CHECK (scope IN ('users','groups','channels','chat')),
    chat_key      INTEGER NOT NULL DEFAULT 0,         -- 0 salvo scope='chat'
    mute_until    INTEGER NOT NULL DEFAULT 0,
    show_previews INTEGER NOT NULL DEFAULT 1 CHECK (show_previews IN (0,1)),
    sound         TEXT,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (scope, chat_key),
    CHECK ((scope = 'chat') = (chat_key <> 0))
) WITHOUT ROWID;

-- Subidas reanudables (F10): estado por partes para continuar tras cerrar la app.
CREATE TABLE uploads (
    local_id     INTEGER PRIMARY KEY,
    chat_key     INTEGER REFERENCES dialogs(chat_key) ON DELETE CASCADE,
    secret_chat_id INTEGER REFERENCES secret_chats(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    size         INTEGER NOT NULL CHECK (size >= 0),
    file_id      INTEGER NOT NULL,                    -- id aleatorio de subida de la red
    part_size    INTEGER NOT NULL CHECK (part_size > 0),
    parts_total  INTEGER NOT NULL,
    parts_done   INTEGER NOT NULL DEFAULT 0,
    state        TEXT NOT NULL DEFAULT 'queued'
                 CHECK (state IN ('queued','uploading','paused','done','failed','cancelled')),
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    CHECK ((chat_key IS NULL) <> (secret_chat_id IS NULL))
);
CREATE INDEX uploads_state ON uploads(state);
CREATE INDEX uploads_chat ON uploads(chat_key);
CREATE INDEX uploads_secret ON uploads(secret_chat_id);

PRAGMA user_version = 1;
