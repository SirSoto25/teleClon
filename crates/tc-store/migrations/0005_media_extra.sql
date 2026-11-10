-- Parlo · caché local SQLite · migración 0005 (PRAGMA user_version = 5)
-- Datos extra de los medios que `media` no recoge (ver crates/tc-store/src/audio.rs y ADR-0007).
--   * media_extra : uno por nota de voz, audio o vídeo redondo.
--       - voice/title/performer/waveform/mime: datos de audio. `waveform` son los valores ya
--         decodificados (0..=31, un byte por barra). `media.mime` de la 0001 sigue sin usarse.
--       - round = 1: vídeo redondo. `media.kind` solo admite 'video' (CHECK de la 0001) y cambiarlo
--         obligaría a reconstruir la tabla; se guarda como 'video' y esta marca lo distingue.

CREATE TABLE media_extra (
    media_id  INTEGER PRIMARY KEY REFERENCES media(id) ON DELETE CASCADE,
    voice     INTEGER NOT NULL DEFAULT 0 CHECK (voice IN (0, 1)),
    round     INTEGER NOT NULL DEFAULT 0 CHECK (round IN (0, 1)),
    has_audio INTEGER NOT NULL DEFAULT 0 CHECK (has_audio IN (0, 1)),
    title     TEXT,
    performer TEXT,
    waveform  BLOB    NOT NULL DEFAULT x'',
    mime      TEXT
);
