# ADR-0006: Empaquetado multiplataforma (Windows, macOS, Linux)

- **Fecha:** 2026-11-07 · **Estado:** aceptado (sin probar en Windows/macOS reales; ver [packaging.md](../packaging.md)).
- **Contexto:** Parlo nació para Linux; se quiere distribuir también `parlo.exe` + instalador y `Parlo.app` + `.dmg`
  (Intel y Apple Silicon). Solo hay una máquina Linux; la compilación real se hace en GitHub Actions.

## Decisiones
1. **Un solo binario por SO, sin dependencias de sistema.** SQLite empaquetado, llavero con `keyring` 4.x en sus
   features por defecto (`v1`: Credential Manager, Keychain, Secret Service por zbus), wgpu con su backend nativo
   (DX12/Vulkan, Metal, Vulkan/GL) y respaldo tiny-skia. `lottie` (rlottie, C++) sigue opcional y fuera de los paquetes.
2. **Directorio de datos según el SO** (`tc_store::default_data_dir`, `directories::BaseDirs`, ya era dependencia):
   `~/.local/share/parlo` (XDG), `%APPDATA%\Parlo`, `~/Library/Application Support/Parlo`. Antes, en Windows,
   `ProjectDirs` daba `%APPDATA%\parlo\data`; no hubo versiones publicadas, así que no se migra. La migración desde
   `teleclon` (ADR-0005) se conserva. El `.env` también se busca en el directorio de datos.
3. **Límite de Windows Credential Manager (2560 bytes por credencial):** la sesión serializada lo supera. Se añade
   `tc_store::ChunkedVault`, que reparte valores grandes en `<clave>.part<g>-<i>` con una cabecera en `<clave>` y
   alterna generaciones para que una escritura interrumpida no deje la cabecera apuntando a trozos a medias.
   Solo reparte en Windows (`KEYRING_MAX_ENTRY`); los valores pequeños se guardan tal cual (compatible con lo ya guardado).
4. **Windows:** `winresource` (dependencia de build solo en `cfg(windows)`) incrusta icono, manifiesto
   (PerMonitorV2, UTF-8, rutas largas, Common Controls v6) y metadatos de versión; `windows_subsystem = "windows"`
   solo en release. Instalador **Inno Setup 6** por usuario (sin UAC) + zip portable. Descartados: WiX/MSI (más
   complejo, no hace falta todavía) y NSIS (script menos legible).
5. **macOS:** `Parlo.app` montado a mano (Info.plist con la versión de Cargo, bundle id PROVISIONAL `app.parlo.Parlo`),
   binario universal con `lipo` desde un runner arm64, `.dmg` con `hdiutil` (sin `create-dmg`, que es una
   dependencia más). Hardened runtime con entitlements vacíos; sin App Sandbox. Mínimo macOS 11.
6. **Iconos PROVISIONALES** generados por `tools/packaging/make_icons.py` (solo biblioteca estándar: escribe PNG, ICO
   e ICNS sin Pillow ni `iconutil`) y versionados. El logo real sustituirá los archivos con los mismos nombres.
7. **CI/CD:** `ci.yml` (build + test + clippy `-D warnings` en ubuntu/windows/macos) y `release.yml` (paquetes como
   artefactos; con etiqueta `v*`, Release en **borrador** con `gh release create` y `SHA256SUMS.txt`). Acciones fijadas
   por versión mayor (`actions/checkout@v4`, `Swatinem/rust-cache@v2`, `actions/{upload,download}-artifact@v4`);
   toolchain con `rustup` (1.99.1) en vez de una acción de terceros. Firma Authenticode, Developer ID y notarización
   solo si existen los secretos; si no, se publica sin firmar.

## Consecuencias
- Sin firma, SmartScreen y Gatekeeper avisarán (documentado en packaging.md).
- Cambiar el bundle id o el `AppId` de Inno Setup más tarde rompe actualizaciones/acceso al llavero: fijarlos al
  verificar el dominio.
- AppImage/Flatpak quedan pendientes.
