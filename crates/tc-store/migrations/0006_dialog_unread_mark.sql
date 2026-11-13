-- Parlo · caché local SQLite · migración 0006 (PRAGMA user_version = 6)
-- Marca manual de «no leído» de un chat (`unread_mark` de la red, `messages.markDialogUnread`).
-- Es independiente de `unread_count`: un chat leído puede estar marcado como no leído. Se quita al
-- leer el chat. Columna nueva con valor por defecto: no hace falta reconstruir la tabla.

ALTER TABLE dialogs ADD COLUMN unread_mark INTEGER NOT NULL DEFAULT 0 CHECK (unread_mark IN (0, 1));
