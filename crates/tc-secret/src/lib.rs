//! Chats secretos E2E según la documentación pública de MTProto 2.0 (end-to-end):
//! intercambio Diffie-Hellman, derivación de `msg_key`/AES-256-IGE, números de secuencia,
//! re-keying con PFS, temporizador de autodestrucción y huella visual de la clave.
//! Diseño sans-IO: no hace red ni disco. Ver docs/architecture.md §6 y ADR-0004.
//!
//! EXPERIMENTAL: criptografía propia sin auditoría externa.
//!
//! Esqueleto: aún sin funcionalidad.
#![forbid(unsafe_code)]
