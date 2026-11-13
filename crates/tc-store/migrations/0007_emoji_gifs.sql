-- Parlo · caché local SQLite · migración 0007 (PRAGMA user_version = 7)
-- Panel del compositor (ADR-0009 emojis, ADR-0010 GIFs; ver crates/tc-store/src/gifs.rs).
--   * emoji_usage : recientes y frecuentes del selector de emojis (solo local: la red no los guarda).
--   * saved_gifs  : última lista de GIFs guardados de la cuenta (`messages.getSavedGifs`), para
--                   pintar el panel sin red. Se reescribe entera en cada carga correcta.

CREATE TABLE emoji_usage (
    emoji     TEXT    PRIMARY KEY CHECK (length(emoji) BETWEEN 1 AND 32),
    uses      INTEGER NOT NULL DEFAULT 0 CHECK (uses >= 0),
    last_used INTEGER NOT NULL
);
CREATE INDEX emoji_usage_last ON emoji_usage (last_used DESC);

CREATE TABLE saved_gifs (
    id             INTEGER PRIMARY KEY,          -- id del documento
    position       INTEGER NOT NULL,             -- 0 = el más reciente
    mime           TEXT    NOT NULL,
    width          INTEGER NOT NULL DEFAULT 0,
    height         INTEGER NOT NULL DEFAULT 0,
    duration_secs  INTEGER,
    title          TEXT    NOT NULL DEFAULT '',
    access_hash    INTEGER NOT NULL,
    file_reference BLOB    NOT NULL DEFAULT x'',
    dc_id          INTEGER NOT NULL,
    size_bytes     INTEGER NOT NULL DEFAULT 0,
    thumb          TEXT
);
CREATE INDEX saved_gifs_position ON saved_gifs (position);
