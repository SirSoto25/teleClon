#!/usr/bin/env bash
# Instala el lanzador (.desktop) y los iconos de Parlo para el usuario actual, sin root.
# Hace falta en Wayland: el compositor no recibe el icono de la ventana, lo busca por el
# application_id («parlo») en parlo.desktop; sin él KDE muestra su icono genérico («W»).
#
# Uso: packaging/linux/install-user.sh [ruta-al-binario]
#   Sin argumento, Exec=parlo (el binario debe estar en el PATH, p. ej. tras instalar el tar.gz).
#   Con argumento (p. ej. "$PWD/target/release/parlo"), Exec apunta a ese binario.
# Desinstalar: rm "$XDG_DATA_HOME"/applications/parlo.desktop "$XDG_DATA_HOME"/icons/hicolor/*/apps/parlo{.png,.svg,-symbolic.svg}
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
EXEC="parlo"
if [[ $# -ge 1 ]]; then
  EXEC="$(realpath "$1")"
  [[ -x "$EXEC" ]] || { echo "No es un ejecutable: $EXEC" >&2; exit 1; }
fi

install -d "$DATA/applications"
# Exec con comillas por si la ruta lleva espacios (especificación Desktop Entry).
sed "s|^Exec=.*|Exec=\"${EXEC//|/\\|}\"|" "$ROOT/packaging/linux/parlo.desktop" > "$DATA/applications/parlo.desktop"
chmod 0644 "$DATA/applications/parlo.desktop"
for s in 16 32 48 64 128 256 512; do
  install -D -m 0644 "$ROOT/packaging/icons/parlo-$s.png" "$DATA/icons/hicolor/${s}x${s}/apps/parlo.png"
done
install -D -m 0644 "$ROOT/packaging/icons/parlo.svg" "$DATA/icons/hicolor/scalable/apps/parlo.svg"
install -D -m 0644 "$ROOT/packaging/icons/parlo-symbolic.svg" "$DATA/icons/hicolor/symbolic/apps/parlo-symbolic.svg"

# Refrescar cachés si las herramientas existen (no es obligatorio en GNOME ni en KDE recientes).
command -v update-desktop-database >/dev/null && update-desktop-database -q "$DATA/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$DATA/icons/hicolor" 2>/dev/null || true
for k in kbuildsycoca6 kbuildsycoca5; do
  if command -v "$k" >/dev/null; then "$k" --noincremental >/dev/null 2>&1 || true; break; fi
done
echo "Instalado: $DATA/applications/parlo.desktop (Exec=$EXEC) e iconos hicolor."
