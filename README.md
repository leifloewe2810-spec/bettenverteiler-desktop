# IPAS Bettenverteiler – Desktop-App

Tray-App fuer Mac und Windows. Wrappt die Web-Anwendung
`https://bettenverteiler.lionsgroup-trading.com` und ergaenzt:

- **Tray-Icon** mit Quick-Actions (Oeffnen, Bettenzahl aendern, Schnellsendung, Beenden)
- **Health-Polling** alle 60 s (Online/Offline-Indikator im Tray-Tooltip)
- **Push-Notification** nach jedem erfolgreichen Versand
- **Close-to-Tray** (Fenster zu = im Hintergrund weiter, Beenden nur via Menue)
- **DevTools** mit `Cmd/Ctrl+Option+I` (zum Debuggen, falls die Webview leer ist)

## Architektur

- Tauri 2 (Rust + WebView)
- Hauptfenster laedt direkt die Server-URL — keine lokale Frontend-Dist noetig
- Hintergrund-Tasks via `tokio::spawn`

## Lokal entwickeln

```bash
npm install
npm run dev      # Tauri-Dev-Mode mit Hot-Reload
```

## Lokal bauen (nur fuer die eigene Plattform)

```bash
./build-dist.sh
# DMG / EXE landet in dist/
```

## Release ueber GitHub Actions (empfohlen)

GitHub Actions baut bei jedem Tag automatisch **Mac (ARM + Intel) + Windows**
und legt ein Release mit allen drei Binaries an.

```bash
# Version in package.json und src-tauri/tauri.conf.json bumpen, dann:
git tag v0.1.2
git push origin v0.1.2
```

Ca. 10 Minuten spaeter steht das Release unter
`https://github.com/<owner>/<repo>/releases/tag/v0.1.2` mit drei Assets:

- `IPAS-Bettenverteiler-v0.1.2-mac-arm64.dmg`
- `IPAS-Bettenverteiler-v0.1.2-mac-x64.dmg`
- `IPAS-Bettenverteiler-v0.1.2-win-x64.exe`

## Code-Signing

Aktuell **unsigniert**. Beim ersten Start kommt:

- **Mac:** „App von unbekanntem Entwickler" → Rechtsklick → „Oeffnen"
- **Windows:** „SmartScreen" → „Weitere Informationen" → „Trotzdem ausfuehren"

Spaeter nachrüstbar mit Apple Developer Account (99 €/Jahr) und Windows
Code-Signing-Zertifikat (~250 €/Jahr).

## Struktur

```
.
├─ src-tauri/
│  ├─ src/
│  │  ├─ main.rs        # Entry-Point (Windows-Subsystem-Flag)
│  │  └─ lib.rs         # Tauri-Setup, Tray, Polling-Loops
│  ├─ capabilities/
│  │  └─ main.json      # Permissions (notification, opener)
│  ├─ icons/            # App-Icons (PNG, ICO, ICNS)
│  ├─ Cargo.toml
│  ├─ build.rs
│  └─ tauri.conf.json   # Window-Config, Bundle-Targets
├─ .github/workflows/
│  └─ release.yml       # CI: Mac + Win Build bei Tag-Push
├─ build-dist.sh        # Lokaler Build-Wrapper
├─ package.json
└─ README.md
```
