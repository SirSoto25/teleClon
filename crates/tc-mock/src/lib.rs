//! `MockBackend`: implementación simulada y determinista de `tc_core::ChatBackend`
//! (semilla fija, latencia configurable, inyección de fallos, mensajes entrantes guionizados,
//! dos cuentas simuladas con cifrado E2E real vía `tc-secret`). Ver docs/architecture.md §5.
//!
//! Esqueleto: aún sin funcionalidad.
#![forbid(unsafe_code)]
