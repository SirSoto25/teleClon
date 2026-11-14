#!/usr/bin/env bash
# Empaqueta Parlo para Linux x86_64 como tar.gz con estructura de prefijo (bin/, share/). ADR-0006.
# Uso: packaging/linux/build-linux.sh   (PARLO_SKIP_BUILD=1 reutiliza target/release/parlo)
# Instalar en el usuario:  tar -xzf Parlo-*.tar.gz -C ~/.local --strip-components=1
# AppImage/Flatpak: pendientes (ver docs/packaging.md).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
VERSION="$(cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "tc-app"))')"
ARCH="$(uname -m)"
if [[ "${PARLO_SKIP_BUILD:-0}" != 1 ]]; then
  cargo build --release --locked -p tc-app
fi

NAME="Parlo-$VERSION-linux-$ARCH"
STAGE="$ROOT/dist/staging/$NAME"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/share/applications" "$STAGE/share/doc/parlo/licenses"
install -m 0755 target/release/parlo "$STAGE/bin/parlo"
install -m 0644 packaging/linux/parlo.desktop "$STAGE/share/applications/parlo.desktop"
for s in 16 32 48 64 128 256 512; do
  install -D -m 0644 "packaging/icons/parlo-$s.png" "$STAGE/share/icons/hicolor/${s}x${s}/apps/parlo.png"
done
install -D -m 0644 packaging/icons/parlo.svg "$STAGE/share/icons/hicolor/scalable/apps/parlo.svg"
install -D -m 0644 packaging/icons/parlo-symbolic.svg "$STAGE/share/icons/hicolor/symbolic/apps/parlo-symbolic.svg"
install -m 0644 README.md "$STAGE/share/doc/parlo/README.md"
install -m 0644 assets/fonts/NOTICE.md "$STAGE/share/doc/parlo/licenses/FONTS-NOTICE.md"
for d in assets/fonts/*/; do
  install -m 0644 "$d/OFL.txt" "$STAGE/share/doc/parlo/licenses/OFL-$(basename "$d").txt"
done
install -m 0644 assets/icons/NOTICE.md "$STAGE/share/doc/parlo/licenses/ICONS-NOTICE.md"
install -m 0644 assets/icons/LICENSE-MIT.txt "$STAGE/share/doc/parlo/licenses/ICONS-LICENSE-MIT.txt"
install -m 0644 assets/licenses/OpenH264-BSD-2-Clause.txt "$STAGE/share/doc/parlo/licenses/OpenH264-BSD-2-Clause.txt"

tar -C "$ROOT/dist/staging" -czf "$ROOT/dist/$NAME.tar.gz" "$NAME"
echo "tar.gz: $ROOT/dist/$NAME.tar.gz"
