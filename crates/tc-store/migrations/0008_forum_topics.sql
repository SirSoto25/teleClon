-- Parlo · caché local SQLite · migración 0008 (PRAGMA user_version = 8)
-- Grupos con temas («comunidades», supergrupos con forum = true). Ver crates/tc-store/src/forum.rs.
--   * groups.is_forum : el grupo se abre en la lista de temas.
--   * forum_topics    : última lista de temas conocida de cada foro (para pintarla sin red).
--   * message_topics  : a qué tema pertenece cada mensaje cacheado de un foro (historial por tema
--                       sin red). Aparte de `messages` para no tocar su esquema; se borra en cascada.

ALTER TABLE groups ADD COLUMN is_forum INTEGER NOT NULL DEFAULT 0 CHECK (is_forum IN (0, 1));

CREATE TABLE forum_topics (
    group_id           INTEGER NOT NULL,                  -- groups.id (sin FK: el tema puede llegar antes que el grupo)
    topic_id           INTEGER NOT NULL,                  -- id del mensaje superior (1 = General)
    title              TEXT    NOT NULL,
    icon_color         INTEGER NOT NULL DEFAULT 0,
    icon_emoji         TEXT,
    custom_emoji_id    INTEGER,
    created_at         INTEGER NOT NULL DEFAULT 0,
    last_msg_id        INTEGER,                           -- último mensaje del tema (en `messages`)
    activity           INTEGER NOT NULL DEFAULT 0,        -- para ordenar sin cargar el mensaje
    unread_count       INTEGER NOT NULL DEFAULT 0 CHECK (unread_count >= 0),
    unread_mentions    INTEGER NOT NULL DEFAULT 0 CHECK (unread_mentions >= 0),
    read_inbox_max_id  INTEGER NOT NULL DEFAULT 0,
    read_outbox_max_id INTEGER NOT NULL DEFAULT 0,
    pinned             INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
    closed             INTEGER NOT NULL DEFAULT 0 CHECK (closed IN (0, 1)),
    hidden             INTEGER NOT NULL DEFAULT 0 CHECK (hidden IN (0, 1)),
    my                 INTEGER NOT NULL DEFAULT 0 CHECK (my IN (0, 1)),
    PRIMARY KEY (group_id, topic_id)
);
CREATE INDEX forum_topics_order ON forum_topics (group_id, pinned DESC, activity DESC);

CREATE TABLE message_topics (
    chat_key INTEGER NOT NULL,
    msg_id   INTEGER NOT NULL,
    topic_id INTEGER NOT NULL,
    PRIMARY KEY (chat_key, msg_id),
    FOREIGN KEY (chat_key, msg_id) REFERENCES messages (chat_key, msg_id) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE INDEX message_topics_topic ON message_topics (chat_key, topic_id, msg_id);
