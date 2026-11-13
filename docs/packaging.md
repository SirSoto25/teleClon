# Empaquetado y releases (Windows, macOS, Linux)

Decisión: [ADR-0006](adr/0006-empaquetado.md). Archivos: `packaging/`, `tools/packaging/make_icons.py`,
`.github/workflows/{ci,release}.yml`, `crates/tc-app/build.rs`.

| SO | Resultado | Script |
| --- | --- | --- |
| Windows 10/11 x86_64 | `parlo.exe` (icono, manifiesto, sin consola en release), `Parlo-<v>-windows-x86_64-portable.zip`, `Parlo-<v>-windows-x86_64-setup.exe` (Inno Setup 6) | `packaging/windows/build.ps1` |
| macOS 11+ (Apple Silicon e Intel) | `Parlo.app` con binario universal (`lipo`), `Parlo-<v>-macos-universal.dmg` (`hdiutil`) | `packaging/macos/build-macos.sh` |
| Linux x86_64 | `Parlo-<v>-linux-x86_64.tar.gz` (bin/, `.desktop`, iconos hicolor, licencias) | `packaging/linux/build-linux.sh` |

La versión sale siempre de `Cargo.toml` (`[workspace.package] version`). Todo va a `dist/` (ignorado por git).

## Compilar en local

Requisito común: Rust 1.99.1 (`rustup toolchain install 1.99.1`). Stickers TGS animados (rlottie, C++,
ADR-0008): **solo en Linux** y siempre activados ahí (dependencia por destino en `tc-ui`); en Windows y
macOS los TGS se dibujan con su imagen estática.

### Linux
rlottie (stickers TGS animados): `rlottie-sys` **clona** rlottie de GitHub con `git` durante el build
(hace falta red la primera vez), lo compila con `cmake` + `g++` (usa `ninja` si está) y genera los
enlaces con bindgen, que necesita libclang (`cmake g++ git libclang-dev` en Debian/Ubuntu; `cmake clang`
en Arch; `cmake gcc-c++ clang-devel` en Fedora). Se enlaza estático (no hay `.so` de rlottie que instalar).

El audio (notas de voz) sale por ALSA vía cpal: para compilar hacen falta las cabeceras
(`libasound2-dev` + `pkg-config` en Debian/Ubuntu, `alsa-lib` en Arch, `alsa-lib-devel` en Fedora) y,
para ejecutar, `libasound.so.2` (presente en cualquier escritorio; PipeWire/PulseAudio la atienden por
su capa ALSA). Windows y macOS no necesitan nada (WASAPI / CoreAudio). Ver ADR-0007.
```sh
cargo build --release -p tc-app          # target/release/parlo
packaging/linux/build-linux.sh            # dist/Parlo-<v>-linux-x86_64.tar.gz
tar -xzf dist/Parlo-*-linux-*.tar.gz -C ~/.local --strip-components=1   # instala en ~/.local
```

### Windows (PowerShell)
Requisitos: Visual Studio Build Tools con «Desarrollo para el escritorio con C++» (MSVC + Windows SDK:
aporta `rc.exe`, que usa `build.rs` para el icono y el manifiesto) e [Inno Setup 6](https://jrsoftware.org/isinfo.php).
```powershell
./packaging/windows/build.ps1                 # exe + zip + instalador
./packaging/windows/build.ps1 -SkipInstaller  # sin Inno Setup: solo exe + zip
```
En debug (`cargo run`) se abre consola para ver los registros; en release no (`windows_subsystem = "windows"`).

### macOS
Requisitos: Xcode o Command Line Tools (`xcode-select --install`), rustup.
```sh
packaging/macos/build-macos.sh                                    # universal (arm64 + x86_64)
PARLO_MAC_TARGETS=aarch64-apple-darwin packaging/macos/build-macos.sh   # solo Apple Silicon, más rápido
open dist/macos/Parlo.app
```

### Icono PROVISIONAL
El logo oficial no existe (solo el brief de [brand.md §5](brand/brand.md)). `python3 tools/packaging/make_icons.py`
genera un marcador geométrico (solo biblioteca estándar) en `packaging/icons/*.png`,
`packaging/windows/parlo.ico` y `packaging/macos/Parlo.icns`; los archivos generados se versionan. Para el
logo definitivo, sustituir esos archivos con los mismos nombres.

## Dónde guarda Parlo los datos

| SO | Datos (`tc_store::default_data_dir`) | Llavero |
| --- | --- | --- |
| Linux | `$XDG_DATA_HOME/parlo` o `~/.local/share/parlo` | Secret Service (D-Bus: GNOME Keyring, KWallet…) |
| Windows | `%APPDATA%\Parlo` | Administrador de credenciales |
| macOS | `~/Library/Application Support/Parlo` | Llavero (Keychain) |

* El `.env` se busca en `TC_ENV_FILE`, luego `./.env` y luego `<datos>/.env` (al abrir la app desde el menú,
  el Explorador o el Finder el directorio actual no es el del proyecto).
* Los permisos 0600 solo se aplican en Unix. En Windows los archivos heredan la ACL de `%APPDATA%` (solo el
  usuario y administradores); el respaldo en claro de la sesión solo se usa si no hay llavero, cosa rara en Windows.
* Windows limita cada credencial a 2560 bytes: `ChunkedVault` reparte la sesión en varias entradas
  `parlo/<cuenta>` (`session`, `session.part<g>-<i>`). En Linux y macOS no se reparte.
* Desinstalar no borra datos ni llavero.

## Disparar un release

1. Subir la versión en `Cargo.toml` (`[workspace.package] version`) y en `packaging/windows/parlo.manifest`
   (`assemblyIdentity version`, opcional), con entrada en `docs/logs/log.md`.
2. Etiquetar y subir: `git tag v0.0.2 && git push origin v0.0.2`.
3. `release.yml` comprueba que la etiqueta coincide con la versión de Cargo, construye en
   `windows-latest`, `macos-latest` y `ubuntu-latest`, y crea un **Release en borrador** con el instalador, el
   zip portable, el dmg, el tar.gz y `SHA256SUMS.txt`. Revisarlo y publicarlo a mano en GitHub.

Sin etiqueta: *Actions → Release → Run workflow* construye los paquetes y los deja como artefactos del run,
sin crear Release. `ci.yml` (build + test + clippy `-D warnings` en los tres SO) corre en cada push a `main` y PR.

## Firma (opcional, por secretos del repositorio)

Sin secretos todo se construye **sin firmar** y el workflow no falla.

| Secreto | Uso |
| --- | --- |
| `WINDOWS_CERT_PFX_BASE64` | Certificado Authenticode (.pfx) en base64 (`base64 -w0 cert.pfx`) |
| `WINDOWS_CERT_PASSWORD` | Contraseña del .pfx. Opcional: variable de entorno `WINDOWS_TIMESTAMP_URL` (por defecto DigiCert) |
| `MACOS_CERT_P12_BASE64` | Certificado «Developer ID Application» exportado como .p12, en base64 |
| `MACOS_CERT_PASSWORD` | Contraseña del .p12 |
| `MACOS_SIGN_IDENTITY` | Opcional: nombre exacto de la identidad; si falta se detecta en el .p12 |
| `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD` | Notarización con `xcrun notarytool` (contraseña de app de appleid.apple.com); requiere firma Developer ID |

* **Windows:** `signtool` firma `parlo.exe` y el instalador (SHA-256 con sello de tiempo RFC 3161). Los
  certificados EV/OV en HSM o en la nube (Azure Trusted Signing, etc.) necesitan adaptar `Sign-File` en `build.ps1`.
* **macOS:** se firma con *hardened runtime* (`codesign -o runtime`) y `packaging/macos/Parlo.entitlements`
  (vacío a propósito), se firma el dmg, se notariza y se grapa el ticket (`stapler`). Sin identidad se firma
  *ad hoc* (necesario para que el binario arm64 se ejecute).

### Avisos sin firma
* **Windows SmartScreen:** «Windows protegió su PC» → *Más información* → *Ejecutar de todas formas*. Incluso
  firmado con un certificado OV, SmartScreen avisa hasta que el certificado gana reputación.
* **macOS Gatekeeper:** sin notarizar, «no se puede abrir porque Apple no puede comprobar…». Abrir con clic
  derecho → *Abrir*, o en macOS 15+ *Ajustes del Sistema → Privacidad y seguridad → Abrir igualmente*. Desde
  terminal: `xattr -dr com.apple.quarantine /Applications/Parlo.app`.

## Qué está verificado y qué no (2026-11-07)

Verificado en Linux (CachyOS, Rust 1.99.1): `cargo build/test/clippy -D warnings` del workspace;
`cargo check --workspace --all-targets --target x86_64-pc-windows-msvc` y `--target aarch64-apple-darwin`
con `RUSTFLAGS=-D warnings` (con un compilador C ficticio para `libsqlite3-sys`: solo comprueba el código
Rust, no compila SQLite ni enlaza); `packaging/linux/build-linux.sh`; iconos (.ico/.icns abiertos con Pillow);
YAML de los workflows (`yaml.safe_load`); XML de plist/manifiesto (`xmllint`); `.desktop`
(`desktop-file-validate`); sintaxis bash (`bash -n`).

**NO probado** (no hay Windows ni macOS aquí, no se ejecutó ningún workflow):
* Compilar/enlazar en Windows y macOS de verdad, `build.rs` con `rc.exe` (icono y manifiesto incrustados),
  `build.ps1`, Inno Setup, `build-macos.sh`, `lipo`, `codesign`, `hdiutil`, notarización, firma Authenticode.
* Que la app arranque y pinte en Windows (DX12/Vulkan) y macOS (Metal), el llavero real de Windows y macOS
  (incluido `ChunkedVault` contra Credential Manager) y las rutas de datos reales.
* `actionlint` no está instalado: los workflows solo se validaron como YAML.
* Tests en CI de Windows/macOS: se espera que pasen (las pruebas de permisos 0600 ya están tras `cfg(unix)`),
  pero no se han ejecutado.

## Pendiente
* Logo definitivo (los iconos son PROVISIONALES) y confirmar el bundle id `app.parlo.Parlo` cuando se
  verifique el dominio (ADR-0005). Cambiar el bundle id más tarde pierde el acceso a lo guardado en el Keychain.
* Linux: AppImage (requiere `appimagetool`/`linuxdeploy`, no trivial sin descargas) y Flatpak (necesita id verificado).
* Windows ARM64 y `aarch64-unknown-linux-gnu` (posibles con la misma estructura; sin probar).
* Archivos `LICENSE-MIT`/`LICENSE-APACHE` en la raíz: la licencia del workspace es `MIT OR Apache-2.0` pero
  aún no hay texto; los paquetes llevan README y las licencias de fuentes e iconos.
* Stickers TGS animados en Windows/macOS (ADR-0008): `rlottie` 0.5.4 no compila en Windows (usa
  `std::os::unix`) y su build.rs enlaza `-lstdc++` fijo (en macOS es libc++). Haría falta un parche o
  fork del crate (o un renderizador Lottie en Rust puro).
