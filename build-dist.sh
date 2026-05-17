#!/usr/bin/env bash
# Baut Distributionen fuer Mac (DMG) und Windows (NSIS-EXE) und legt sie
# in dist/ ab. Beide unsigned – Empfaenger muessen einmalig die Warnung
# umgehen (Mac: Rechtsklick -> Oeffnen, Win: "Trotzdem ausfuehren").
#
# Voraussetzungen:
#   - macOS-Build:  Mac mit Xcode CLT
#   - Windows-Build: Rust-Targets x86_64-pc-windows-msvc + WiX (kann auf
#     einer Win-Maschine laufen, oder cross via Docker/CI)
#
# Mit dieser Lokalvariante bauen wir nur das, was die aktuelle Plattform
# liefern kann.

set -e
cd "$(dirname "$0")"
mkdir -p dist

VERSION=$(node -p "require('./package.json').version")
PLAT=$(uname -s)

echo "=== Bettenverteiler-Desktop $VERSION ==="

if [[ "$PLAT" == "Darwin" ]]; then
    echo
    echo "--- Tauri-Build (Mac, .app) ---"
    npx tauri build --bundles app
    APP="src-tauri/target/release/bundle/macos/IPAS Bettenverteiler.app"
    if [ ! -d "$APP" ]; then
        echo "FEHLER: .app nicht gefunden unter $APP"
        exit 1
    fi

    echo
    echo "--- DMG bauen (hdiutil) ---"
    OUT="dist/IPAS-Bettenverteiler-${VERSION}-mac.dmg"
    rm -f "$OUT"
    hdiutil create -volname "IPAS Bettenverteiler" \
        -srcfolder "$APP" \
        -ov -format UDZO \
        "$OUT"
    echo "OK: $OUT ($(du -h "$OUT" | cut -f1))"

    echo
    echo "--- Hinweis fuer Empfaenger ---"
    cat <<EOF
Mac-Nutzer beim ersten Start:
  1. DMG oeffnen, App ins Programme-Verzeichnis ziehen.
  2. App im Finder mit Rechtsklick -> "Oeffnen" starten.
  3. Im Dialog "Oeffnen" bestaetigen.
Beim zweiten Start funktioniert Doppelklick normal.
EOF

elif [[ "$PLAT" == MINGW* ]] || [[ "$PLAT" == CYGWIN* ]] || [[ "$PLAT" == MSYS* ]]; then
    echo
    echo "--- Tauri-Build (Windows, NSIS) ---"
    npx tauri build --bundles nsis
    OUT="src-tauri/target/release/bundle/nsis/"
    cp "$OUT"/*-setup.exe "dist/IPAS-Bettenverteiler-${VERSION}-win.exe"
    echo "OK: dist/IPAS-Bettenverteiler-${VERSION}-win.exe"

    echo
    cat <<EOF
Windows-Nutzer beim ersten Start:
  1. EXE doppelklicken.
  2. SmartScreen-Warnung: "Weitere Informationen" -> "Trotzdem ausfuehren".
  3. Installer abschliessen.
EOF

else
    echo "Plattform $PLAT wird vom Lokal-Build nicht unterstuetzt."
    echo "Fuer Linux: 'npx tauri build --bundles deb' selbst aufrufen."
    exit 1
fi

echo
echo "=== Fertig ==="
ls -lh dist/
