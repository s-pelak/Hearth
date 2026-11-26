//! Hearth desktop shell: tray / menu bar icon, main window, and commands into hearth-core.

use hearth_core::{ProviderConfig, ProviderKind};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, WindowEvent};

mod native;
use native::NativeState;

#[derive(Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    kind: ProviderKind,
    base_url: String,
    is_local: bool,
}

/// Configured providers. Phase 1 reads defaults; Settings will persist edits.
#[tauri::command]
fn list_providers() -> Vec<ProviderInfo> {
    ProviderConfig::defaults()
        .into_iter()
        .map(|p| ProviderInfo { id: p.id, name: p.name, kind: p.kind, base_url: p.base_url, is_local: p.is_local })
        .collect()
}

/// Ask a provider which models it serves (local providers need no key).
#[tauri::command]
async fn list_models(provider_id: String) -> Result<Vec<String>, String> {
    let cfg = ProviderConfig::defaults().into_iter().find(|p| p.id == provider_id).ok_or("unknown provider")?;
    cfg.build(None).list_models().await.map_err(|e| e.to_string())
}

pub(crate) fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    native::sync_activation_policy(app);
}

fn toggle_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
            let _ = w.hide();
            native::sync_activation_policy(app);
        } else {
            show_main(app);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init()).manage(NativeState::default());

    // The full menu bar is a macOS convention. Windows apps following Fluent
    // have no menu bar; their shortcuts are handled in the UI.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(native::app_menu).on_menu_event(|app, e| native::on_menu(app, e.id.as_ref()));

    builder
        .setup(|app| {
            if let Some(main) = app.get_webview_window("main") {
                let material = native::apply_material(&main);
                *app.state::<NativeState>().material.lock().unwrap() = material;
            }
            // The window opens on launch, so start as a regular app; hiding
            // the last window drops back to the menu bar only.
            native::sync_activation_policy(app.handle());

            let open = MenuItem::with_id(app, "open", "Open Hearth", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Hearth", true, Some("CmdOrCtrl+Q"))?;
            let menu = Menu::with_items(app, &[&open, &quit])?;

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
            TrayIconBuilder::with_id("hearth")
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Hearth")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, e| native::on_menu(app, e.id.as_ref()))
                .on_tray_icon_event(|tray, e| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                        toggle_main(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps Hearth running in the menu bar / tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            if matches!(event, WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed) {
                native::sync_activation_policy(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![list_providers, list_models, native::window_chrome, native::open_settings])
        .run(tauri::generate_context!())
        .expect("error while running Hearth");
}
