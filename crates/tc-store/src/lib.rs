//! Caché local SQLite (un archivo por cuenta, WAL, `secure_delete`), migraciones con
//! `PRAGMA user_version` (ver `migrations/`), `CachedBackend` (decorador de `ChatBackend`)
//! y bóveda de claves sobre el keyring del sistema. Ver docs/architecture.md §7.
//!
//! Esqueleto: aún sin funcionalidad.
#![forbid(unsafe_code)]
