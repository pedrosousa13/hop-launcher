#!/usr/bin/env bash
set -euo pipefail

# Build AppImage for hop-launcher-gtk from this repository.
# Output: dist/hop-launcher-gtk-linux-x86_64.AppImage

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist"
APPDIR="${ROOT_DIR}/dist/AppDir"
TOOLS_DIR="${ROOT_DIR}/dist/tools"

mkdir -p "${DIST_DIR}" "${TOOLS_DIR}"

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

LINUXDEPLOY="${TOOLS_DIR}/linuxdeploy-x86_64.AppImage"
APPIMAGETOOL="${TOOLS_DIR}/appimagetool-x86_64.AppImage"

if [[ ! -x "${LINUXDEPLOY}" ]]; then
  curl -fsSL -o "${LINUXDEPLOY}" \
    "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
  chmod +x "${LINUXDEPLOY}"
fi

if [[ ! -x "${APPIMAGETOOL}" ]]; then
  curl -fsSL -o "${APPIMAGETOOL}" \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"
  chmod +x "${APPIMAGETOOL}"
fi

pushd "${ROOT_DIR}" >/dev/null
cargo build --release --features gtk_ui --manifest-path apps/gtk-launcher/Cargo.toml
popd >/dev/null

rm -rf "${APPDIR}"
mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/share/applications" "${APPDIR}/usr/share/icons/hicolor/256x256/apps"

install -m 0755 "${ROOT_DIR}/apps/gtk-launcher/target/release/hop-launcher-gtk" "${APPDIR}/usr/bin/hop-launcher-gtk"
install -m 0644 "${ROOT_DIR}/packaging/appimage/hop-launcher.desktop" "${APPDIR}/usr/share/applications/hop-launcher.desktop"
install -m 0644 "${ROOT_DIR}/docs-site/public/icons/apps/chromium.png" "${APPDIR}/usr/share/icons/hicolor/256x256/apps/hop-launcher.png"

APPIMAGE_EXTRACT_AND_RUN=1 ARCH=x86_64 "${LINUXDEPLOY}" --appdir "${APPDIR}" \
  --executable "${APPDIR}/usr/bin/hop-launcher-gtk" \
  --desktop-file "${APPDIR}/usr/share/applications/hop-launcher.desktop" \
  --icon-file "${APPDIR}/usr/share/icons/hicolor/256x256/apps/hop-launcher.png" \
  --output appimage

OUTPUT="$(ls -1 "${ROOT_DIR}"/*.AppImage | head -n 1)"
if [[ -z "${OUTPUT}" || ! -f "${OUTPUT}" ]]; then
  echo "AppImage build did not produce an output file" >&2
  exit 1
fi

mv -f "${OUTPUT}" "${DIST_DIR}/hop-launcher-gtk-linux-x86_64.AppImage"
echo "Wrote ${DIST_DIR}/hop-launcher-gtk-linux-x86_64.AppImage"
