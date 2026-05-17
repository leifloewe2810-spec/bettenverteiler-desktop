// IPAS Bettenverteiler Desktop-App
//
// Architektur:
//   - Webview laedt direkt die produktive Web-App
//     (https://bettenverteiler.lionsgroup-trading.com/).
//   - Tray-Icon mit Menue:
//       - "Bettenverteiler oeffnen"  -> blendet Fenster ein, fokussiert
//       - "Bettenzahl aendern…"       -> oeffnet /steuerung
//       - "Schnellsendung…"           -> oeffnet /versand
//       - "Status:"                    -> nicht klickbar, zeigt online/offline
//       - "Beenden"
//   - Hintergrund-Task pollt alle 60 s /api/health (Offline-Indikator + Tray-
//     Tooltip).
//   - Hintergrund-Task pollt alle 5 min /api/sendlog (neueste 1) und schickt
//     eine OS-Notification, sobald ein neuer Lauf erscheint.
//   - Fenster wird beim Schliessen versteckt statt beendet (klassisches Tray-
//     Verhalten); echtes Beenden nur ueber Tray-Menue.

use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;
use tauri::menu::{Menu, MenuEvent, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_notification::NotificationExt;

const BACKEND_BASE: &str = "https://bettenverteiler.lionsgroup-trading.com";
const HEALTH_INTERVAL_SECS: u64 = 60;
const SENDLOG_INTERVAL_SECS: u64 = 300;

#[derive(Default)]
struct AppState {
    /// Letzte gesehene lauf_id; wird genutzt, um neue Versand-Laeufe zu
    /// erkennen und einmal eine Notification zu schicken.
    letzter_lauf_id: Mutex<Option<String>>,
}

#[derive(Debug, Deserialize)]
struct SendLogHeader {
    lauf_id: String,
    #[serde(default)]
    kw: u32,
    #[serde(default)]
    empfaenger_gesamt: u32,
    #[serde(default)]
    erfolgreich: u32,
    #[serde(default)]
    fehler: u32,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(AppState::default())
        .setup(|app| {
            let handle = app.handle().clone();

            // Tray-Menue aufbauen
            let oeffnen = MenuItemBuilder::with_id("open", "Bettenverteiler oeffnen").build(app)?;
            let bett = MenuItemBuilder::with_id("bett", "Bettenzahl aendern…").build(app)?;
            let schnell = MenuItemBuilder::with_id("schnell", "Schnellsendung…").build(app)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let beenden = MenuItemBuilder::with_id("quit", "Beenden").build(app)?;
            let menu = Menu::with_items(
                app,
                &[&oeffnen, &bett, &schnell, &sep, &beenden],
            )?;

            let _tray = TrayIconBuilder::with_id("bv-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("IPAS Bettenverteiler")
                .menu(&menu)
                .on_menu_event(|app, event: MenuEvent| handle_menu(app, event))
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::DoubleClick { .. } => {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            // Hauptfenster: beim Schliessen nur verstecken (Tray-Verhalten)
            if let Some(win) = app.get_webview_window("main") {
                let win_clone = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
                // Window initial gleich zeigen — User erwartet beim ersten Start eine UI
                let _ = win.show();
                let _ = win.set_focus();
            }

            // Hintergrund-Tasks: Health-Polling + Sendlog-Polling
            tauri::async_runtime::spawn(health_loop(handle.clone()));
            tauri::async_runtime::spawn(sendlog_loop(handle));

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri-Setup fehlgeschlagen");
}

fn handle_menu(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "open" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "bett" => navigate(app, "/steuerung"),
        "schnell" => navigate(app, "/versand"),
        "quit" => {
            app.exit(0);
        }
        _ => {}
    }
}

/// Schaltet den Webview auf eine andere Pfad-Route der gleichen Domain und
/// blendet das Fenster ein.
fn navigate(app: &AppHandle, pfad: &str) {
    if let Some(win) = app.get_webview_window("main") {
        let url = format!("{BACKEND_BASE}{pfad}");
        // Per Event an die Webseite navigieren via window.location
        let _ = win.eval(&format!("window.location = '{}';", url.replace('\'', "\\'")));
        let _ = win.show();
        let _ = win.set_focus();
    }
}

async fn health_loop(handle: AppHandle) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .unwrap();
    loop {
        let ok = client
            .get(format!("{BACKEND_BASE}/api/health"))
            .send()
            .await
            .ok()
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if let Some(tray) = handle.tray_by_id("bv-tray") {
            let tooltip = if ok {
                "IPAS Bettenverteiler – online"
            } else {
                "IPAS Bettenverteiler – OFFLINE (Server nicht erreichbar)"
            };
            let _ = tray.set_tooltip(Some(tooltip));
        }
        // Frontend-Update fuer evtl. Anzeige
        let _ = handle.emit("bv-health", ok);
        tokio::time::sleep(Duration::from_secs(HEALTH_INTERVAL_SECS)).await;
    }
}

async fn sendlog_loop(handle: AppHandle) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    loop {
        // Endpoint liefert eine Liste der letzten N Laeufe — wir nehmen den ersten.
        if let Ok(res) = client
            .get(format!("{BACKEND_BASE}/api/sendlog?limit=1"))
            .send()
            .await
        {
            if res.status().is_success() {
                if let Ok(list) = res.json::<Vec<SendLogHeader>>().await {
                    if let Some(neu) = list.into_iter().next() {
                        let state = handle.state::<AppState>();
                        let mut prev = state.letzter_lauf_id.lock().unwrap();
                        let ist_neu = prev.as_deref() != Some(neu.lauf_id.as_str());
                        let erster_durchlauf = prev.is_none();
                        *prev = Some(neu.lauf_id.clone());
                        drop(prev);
                        if ist_neu && !erster_durchlauf {
                            let _ = handle
                                .notification()
                                .builder()
                                .title("Bettenmail versendet")
                                .body(format!(
                                    "KW {}: {} Empfaenger, {} ok, {} Fehler.",
                                    neu.kw, neu.empfaenger_gesamt, neu.erfolgreich, neu.fehler
                                ))
                                .show();
                        }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(SENDLOG_INTERVAL_SECS)).await;
    }
}
