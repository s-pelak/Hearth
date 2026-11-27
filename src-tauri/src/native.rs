//! Platform chrome that the web UI cannot draw itself: window materials,
//! the macOS app menu, the Settings window and Dock / menu bar presence.
//!
//! macOS follows Apple's Human Interface Guidelines (sidebar vibrancy, a full
//! menu bar with standard ⌘ shortcuts, Settings as its own window on ⌘,).
//! Windows follows Fluent (Mica backdrop, no menu bar, Settings as a page in
//! the main window, Ctrl shortcuts handled by the UI).

use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow};

/// Which backdrop the OS actually applied, so the UI only paints itself
/// translucent where something is behind it.
#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(not(any(target_os = "macos", target_os = "windows")), allow(dead_code))]
pub enum Material {
    /// macOS `NSVisualEffectView` sidebar material behind the whole window.
    Sidebar,
    /// Windows 11 Mica.
    Mica,
    /// Nothing applied (Windows 10, Linux, or the call failed): paint opaque.
    #[default]
    None,
}

#[derive(Default)]
pub struct NativeState {
    pub material: Mutex<Material>,
}

#[derive(Serialize)]
pub struct WindowChrome {
    platform: &'static str,
    material: Material,
}

fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    }
}

/// Apply the platform backdrop to a window and report what stuck.
pub fn apply_material<R: Runtime>(window: &WebviewWindow<R>) -> Material {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        if apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, Some(NSVisualEffectState::FollowsWindowActiveState), None)
            .is_ok()
        {
            return Material::Sidebar;
        }
    }
    #[cfg(target_os = "windows")]
    {
        // `None` lets Mica follow the system light/dark setting.
        if window_vibrancy::apply_mica(window, None).is_ok() {
            return Material::Mica;
        }
    }
    let _ = window;
    Material::None
}

/// The UI calls this once on load to pick its platform styles.
#[tauri::command]
pub fn window_chrome(state: tauri::State<'_, NativeState>, window: WebviewWindow) -> WindowChrome {
    // Only the main window gets a backdrop; Settings is an opaque standard window.
    let material = if window.label() == "main" { *state.material.lock().unwrap() } else { Material::None };
    WindowChrome { platform: platform(), material }
}

/// Open Settings the way each platform expects: its own window on macOS,
/// a page inside the main window elsewhere.
#[tauri::command]
pub fn open_settings(app: AppHandle) {
    show_settings(&app);
}

pub fn show_settings<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        if let Some(w) = app.get_webview_window("settings") {
            let _ = w.show();
            let _ = w.set_focus();
            return;
        }
        let _ = tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App("index.html#/settings".into()))
            .title("Settings")
            .inner_size(680.0, 520.0)
            .resizable(false)
            .minimizable(false)
            .maximizable(false)
            .center()
            .build();
    }
    #[cfg(not(target_os = "macos"))]
    {
        crate::show_main(app);
        let _ = app.emit_to("main", "menu", "settings");
    }
}

/// A menu bar app lives in the menu bar only while no window is open. When a
/// window is showing it becomes a regular app with a Dock icon and the menu
/// bar, so ⌘ shortcuts and the Window menu work as on any Mac app.
pub fn sync_activation_policy<R: Runtime>(app: &AppHandle<R>) {
    #[cfg(target_os = "macos")]
    {
        let any_visible = app.webview_windows().values().any(|w| w.is_visible().unwrap_or(false));
        let policy = if any_visible { tauri::ActivationPolicy::Regular } else { tauri::ActivationPolicy::Accessory };
        let _ = app.set_activation_policy(policy);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

/// The macOS menu bar. Custom items are forwarded to the UI as a `menu` event
/// carrying the item id; standard items use AppKit's own actions.
#[cfg(target_os = "macos")]
pub fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem as P, SubmenuBuilder};

    let item = |id: &str, text: &str, accel: &str| MenuItemBuilder::with_id(id, text).accelerator(accel).build(app);

    let hearth = SubmenuBuilder::new(app, "Hearth")
        .item(&P::about(app, Some("About Hearth"), Some(AboutMetadata::default()))?)
        .separator()
        .item(&item("settings", "Settings…", "Cmd+,")?)
        .separator()
        .item(&P::services(app, None)?)
        .separator()
        .item(&P::hide(app, None)?)
        .item(&P::hide_others(app, None)?)
        .item(&P::show_all(app, None)?)
        .separator()
        .item(&P::quit(app, Some("Quit Hearth"))?)
        .build()?;

    let file = SubmenuBuilder::new(app, "File")
        .item(&item("new_thread", "New Thread", "Cmd+N")?)
        .item(&item("new_chat", "New Chat", "Shift+Cmd+N")?)
        .item(&item("new_project", "New Project…", "Alt+Cmd+N")?)
        .separator()
        .item(&item("attach", "Attach Files…", "Shift+Cmd+A")?)
        .separator()
        .item(&P::close_window(app, None)?)
        .build()?;

    let edit = SubmenuBuilder::new(app, "Edit")
        .item(&P::undo(app, None)?)
        .item(&P::redo(app, None)?)
        .separator()
        .item(&P::cut(app, None)?)
        .item(&P::copy(app, None)?)
        .item(&P::paste(app, None)?)
        .item(&P::select_all(app, None)?)
        .separator()
        .item(&item("find", "Find…", "Cmd+F")?)
        .build()?;

    let view = SubmenuBuilder::new(app, "View")
        .item(&item("toggle_sidebar", "Toggle Sidebar", "Ctrl+Cmd+S")?)
        .item(&item("toggle_thread_list", "Toggle Thread List", "Alt+Cmd+L")?)
        .item(&item("toggle_artifacts", "Toggle Artifacts", "Alt+Cmd+A")?)
        .separator()
        .item(&item("overview", "Overview", "Cmd+1")?)
        .item(&item("search", "Search…", "Cmd+K")?)
        .separator()
        .item(&P::fullscreen(app, None)?)
        .build()?;

    let thread = SubmenuBuilder::new(app, "Thread")
        .item(&item("send", "Send", "Cmd+Enter")?)
        .item(&item("stop", "Stop", "Cmd+.")?)
        .separator()
        .item(&item("mark_done", "Mark Done", "Shift+Cmd+Enter")?)
        .item(&item("retry_stronger", "Retry with Stronger Model…", "Shift+Cmd+R")?)
        .item(&item("switch_model", "Switch Model…", "Cmd+M")?)
        .build()?;

    let window = SubmenuBuilder::new(app, "Window")
        .item(&P::minimize(app, None)?)
        .item(&P::maximize(app, Some("Zoom"))?)
        .separator()
        .item(&item("bring_all", "Bring All to Front", "")?)
        .build()?;

    let help = SubmenuBuilder::new(app, "Help").item(&item("help", "Hearth Help", "")?).build()?;

    MenuBuilder::new(app).items(&[&hearth, &file, &edit, &view, &thread, &window, &help]).build()
}

/// Route a menu click: native actions here, everything else to the UI.
pub fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        "settings" => show_settings(app),
        "open" => crate::show_main(app),
        "quit" => app.exit(0),
        "bring_all" => {
            for w in app.webview_windows().values() {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.set_focus();
                }
            }
        }
        other => {
            crate::show_main(app);
            let _ = app.emit_to("main", "menu", other);
        }
    }
}
