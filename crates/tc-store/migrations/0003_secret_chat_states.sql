-- teleClon · caché local SQLite · migración 0003 (PRAGMA user_version = 3)
-- Estado de protocolo de los chats secretos de tc-telegram (`StoredSecretChat`: metadatos + instantánea
-- `tc_secret::SessionSnapshot`, que contiene la clave del chat y textos en claro recientes).
-- Aquí SOLO se guarda el blob ya cifrado con AEAD (AES-256-GCM): la clave de cifrado vive en el keyring
-- (`SecretVault`, nombre "secret-state-key"), nunca en esta base. Sin keyring no se escribe nada.
--   * secret_chat_id : id del chat secreto (también va en los datos asociados del AEAD: un blob movido a
--                      otra fila no descifra).
--   * format         : versión del formato del texto en claro (la decide el adaptador).
--   * nonce          : 12 bytes aleatorios, nuevos en cada escritura.
--   * ciphertext     : texto cifrado + etiqueta de 16 bytes.
-- Sin clave foránea a `secret_chats`: tc-telegram no usa esa tabla (sus ids son los de la red).

CREATE TABLE secret_chat_states (
    secret_chat_id INTEGER PRIMARY KEY,
    format         INTEGER NOT NULL,
    nonce          BLOB    NOT NULL CHECK (length(nonce) = 12),
    ciphertext     BLOB    NOT NULL CHECK (length(ciphertext) >= 16),
    updated_at     INTEGER NOT NULL
);

PRAGMA user_version = 3;
