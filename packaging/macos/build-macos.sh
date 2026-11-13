#!/usr/bin/env bash
# Construye Parlo.app (binario universal arm64 + x86_64) y Parlo-<versión>-macos-universal.dmg. ADR-0006.
#
# Uso (en macOS 12+ con Xcode o Command Line Tools y rustup):
#   packaging/macos/build-macos.sh            # universal
#   PARLO_MAC_TARGETS=aarch64-apple-darwin packaging/macos/build-macos.sh   # solo Apple Silicon
#
# Firma OPCIONAL (si no hay identidad se firma ad hoc y Gatekeeper avisará al abrir):
#   MACOS_SIGN_IDENTITY   p. ej. "Developer ID Application: Nombre (TEAMID)" (ya importada en un llavero)
# Notarización OPCIONAL (requiere firma con Developer ID):
#   APPLE_ID, APPLE_TEAM_ID, APPLE_APP_PASSWORD (contraseña de app de appleid.apple.com)
#
# Salida: dist/macos/Parlo.app y dist/Parlo-<versión>-macos-universal.dmg (o -arm64 / -x86_64).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HERE="$ROOT/packaging/macos"
DIST="$ROOT/dist"
OUT="$DIST/macos"
TARGETS="${PARLO_MAC_TARGETS:-aarch64-apple-darwin x86_64-apple-darwin}"
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"

cd "$ROOT"
VERSION="$(cargo metadata --no-deps --format-version 1 \
  | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "tc-app"))')"
echo "Parlo $VERSION ($TARGETS, macOS >= $MACOSX_DEPLOYMENT_TARGET)"

# --- Compilación y binario universal ---------------------------------------------------------
bins=()
for t in $TARGETS; do
  if command -v rustup >/dev/null; then rustup target add "$t" >/dev/null; fi
  if [[ "${PARLO_SKIP_BUILD:-0}" != 1 ]]; then
    cargo build --release --locked -p tc-app --target "$t"
  fi
  bins+=("$ROOT/target/$t/release/parlo")
done
case "$TARGETS" in
  *aarch64*x86_64*|*x86_64*aarch64*) SUFFIX=universal ;;
  *aarch64*) SUFFIX=arm64 ;;
  *) SUFFIX=x86_64 ;;
esac

rm -rf "$OUT"
APP="$OUT/Parlo.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/licenses"
if [[ ${#bins[@]} -gt 1 ]]; then
  lipo -create -output "$APP/Contents/MacOS/parlo" "${bins[@]}"
else
  cp "${bins[0]}" "$APP/Contents/MacOS/parlo"
fi
lipo -info "$APP/Contents/MacOS/parlo"

# --- Bundle -------------------------------------------------------------------------------
sed "s/@VERSION@/$VERSION/g" "$HERE/Info.plist.in" > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"
cp "$HERE/Parlo.icns" "$APP/Contents/Resources/Parlo.icns"
cp "$ROOT/README.md" "$APP/Contents/Resources/"
cp "$ROOT/assets/fonts/NOTICE.md" "$APP/Contents/Resources/licenses/FONTS-NOTICE.md"
for d in "$ROOT"/assets/fonts/*/; do
  cp "$d/OFL.txt" "$APP/Contents/Resources/licenses/OFL-$(basename "$d").txt"
done
cp "$ROOT/assets/icons/NOTICE.md" "$APP/Contents/Resources/licenses/ICONS-NOTICE.md"
cp "$ROOT/assets/licenses/OpenH264-BSD-2-Clause.txt" "$APP/Contents/Resources/licenses/OpenH264-BSD-2-Clause.txt"
cp "$ROOT/assets/icons/LICENSE-MIT.txt" "$APP/Contents/Resources/licenses/ICONS-LICENSE-MIT.txt"

# --- Firma ----------------------------------------------------------------------------------
IDENTITY="${MACOS_SIGN_IDENTITY:-}"
if [[ -n "$IDENTITY" ]]; then
  echo "Firmando con: $IDENTITY (hardened runtime)"
  codesign --force --timestamp --options runtime --entitlements "$HERE/Parlo.entitlements" \
    --sign "$IDENTITY" "$APP/Contents/MacOS/parlo"
  codesign --force --timestamp --options runtime --entitlements "$HERE/Parlo.entitlements" \
    --sign "$IDENTITY" "$APP"
else
  echo "Sin MACOS_SIGN_IDENTITY: firma ad hoc (Gatekeeper pedirá confirmación al abrir)."
  codesign --force --sign - "$APP"
fi
codesign --verify --strict --verbose=2 "$APP"

# --- DMG -----------------------------------------------------------------------------------
DMG="$DIST/Parlo-$VERSION-macos-$SUFFIX.dmg"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
rm -f "$DMG"
hdiutil create -volname "Parlo" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
if [[ -n "$IDENTITY" ]]; then
  codesign --force --timestamp --sign "$IDENTITY" "$DMG"
fi

# --- Notarización (opcional) ---------------------------------------------------------------
if [[ -n "$IDENTITY" && -n "${APPLE_ID:-}" && -n "${APPLE_TEAM_ID:-}" && -n "${APPLE_APP_PASSWORD:-}" ]]; then
  echo "Notarizando $DMG ..."
  xcrun notarytool submit "$DMG" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_PASSWORD" --wait
  xcrun stapler staple "$DMG"
  xcrun stapler validate "$DMG"
else
  echo "Sin notarizar (faltan MACOS_SIGN_IDENTITY o APPLE_ID/APPLE_TEAM_ID/APPLE_APP_PASSWORD)."
fi
echo "dmg: $DMG"
