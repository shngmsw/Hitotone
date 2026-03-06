use crate::state::AppState;
use tauri::Manager;

const DOCK_WIDTH: f64 = 64.0;
const HEADER_HEIGHT: f64 = 32.0;

pub struct LayoutParams {
    pub service_x: f64,
    pub service_y: f64,
    pub service_width: f64,
    pub service_height: f64,
    pub ai_x: f64,
    pub ai_y: f64,
    pub ai_width: f64,
    pub ai_height: f64,
}

/// Position a child WebviewWindow.
/// After SetParent on Windows, coordinates are relative to parent's client area.
/// On other platforms, we need absolute screen coordinates.
fn position_child(
    #[allow(unused_variables)] main_ww: &tauri::WebviewWindow,
    child_ww: &tauri::WebviewWindow,
    rel_x: f64,
    rel_y: f64,
    width: f64,
    height: f64,
) {
    #[cfg(target_os = "windows")]
    {
        // SetParent makes coordinates relative to parent's client area
        let _ = child_ww.set_position(tauri::LogicalPosition::new(rel_x, rel_y));
        let _ = child_ww.set_size(tauri::LogicalSize::new(width, height));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(inner_pos) = main_ww.inner_position() {
            let scale = main_ww.scale_factor().unwrap_or(1.0);
            let abs_x = inner_pos.x as f64 / scale + rel_x;
            let abs_y = inner_pos.y as f64 / scale + rel_y;
            let _ = child_ww.set_position(tauri::LogicalPosition::new(abs_x, abs_y));
            let _ = child_ww.set_size(tauri::LogicalSize::new(width, height));
        }
    }
}

/// Calculate service and AI layout areas within the main window's client area.
pub fn get_layout_params(main_ww: &tauri::WebviewWindow, state: &AppState) -> Option<LayoutParams> {
    let window_size = main_ww.inner_size().ok()?;
    let scale = main_ww.scale_factor().unwrap_or(1.0);
    let w = window_size.width as f64 / scale;
    let h = window_size.height as f64 / scale;
    let ai_w = if state.show_ai_companion {
        state.ai_width as f64
    } else {
        0.0
    };

    Some(LayoutParams {
        service_x: DOCK_WIDTH,
        service_y: HEADER_HEIGHT,
        service_width: (w - DOCK_WIDTH - ai_w).max(100.0),
        service_height: (h - HEADER_HEIGHT).max(100.0),
        ai_x: w - state.ai_width as f64,
        ai_y: HEADER_HEIGHT,
        ai_width: state.ai_width as f64,
        ai_height: (h - HEADER_HEIGHT).max(100.0),
    })
}

/// OAuth/認証関連のURLかどうかを判定する
fn is_auth_url(url: &str) -> bool {
    url.contains("accounts.google.com")
        || url.contains("login.microsoftonline.com")
        || url.contains("github.com/login/oauth")
        || url.contains("slack.com/sso")
        || url.contains("slack.com/oauth")
        || (url.contains("slack.com") && url.contains("/signin/"))
}

/// Create a service WebviewWindow dynamically.
pub fn create_service_webview_window(
    app: &tauri::AppHandle,
    main_ww: &tauri::WebviewWindow,
    label: &str,
    url: &str,
    layout: &LayoutParams,
) -> Result<tauri::WebviewWindow, String> {
    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;
    let app_for_nav = app.clone();

    // Create hidden, then SetParent, then position with relative coords
    let ww = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::External(parsed_url),
    )
    .title(&format!("Hitotone - {}", label))
    .inner_size(layout.service_width, layout.service_height)
    .decorations(false)
    .skip_taskbar(true)
    .visible(false)
    .on_navigation(move |nav_url| {
        let url_str = nav_url.as_str();
        println!("[service-nav] {}", url_str);
        // OAuth系URLへのナビゲーションを捕捉してポップアップとして開く
        if is_auth_url(url_str) {
            let owned = url_str.to_string();
            let app2 = app_for_nav.clone();
            std::thread::spawn(move || {
                // 少し待ってからポップアップを開く（WebView2のナビゲーションキャンセル後に実行）
                std::thread::sleep(std::time::Duration::from_millis(50));
                crate::commands::open_popup_window_internal(&app2, owned);
            });
            return false; // ナビゲーションをキャンセル
        }
        true // それ以外は通常ナビゲーション
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Apply SetParent on Windows, then position with relative coordinates
    #[cfg(target_os = "windows")]
    {
        set_parent_window(main_ww, &ww);
        let _ = ww.set_position(tauri::LogicalPosition::new(layout.service_x, layout.service_y));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(inner_pos) = main_ww.inner_position() {
            let scale = main_ww.scale_factor().unwrap_or(1.0);
            let abs_x = inner_pos.x as f64 / scale + layout.service_x;
            let abs_y = inner_pos.y as f64 / scale + layout.service_y;
            let _ = ww.set_position(tauri::LogicalPosition::new(abs_x, abs_y));
        }
    }

    Ok(ww)
}

/// Create the AI companion WebviewWindow dynamically.
pub fn create_ai_webview_window(
    app: &tauri::AppHandle,
    main_ww: &tauri::WebviewWindow,
    url: &str,
    layout: &LayoutParams,
) -> Result<tauri::WebviewWindow, String> {
    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let ww = tauri::WebviewWindowBuilder::new(
        app,
        "ai-webview",
        tauri::WebviewUrl::External(parsed_url),
    )
    .title("Hitotone - AI")
    .inner_size(layout.ai_width, layout.ai_height)
    .decorations(false)
    .skip_taskbar(true)
    .visible(false)
    .build()
    .map_err(|e| e.to_string())?;

    #[cfg(target_os = "windows")]
    {
        set_parent_window(main_ww, &ww);
        let _ = ww.set_position(tauri::LogicalPosition::new(layout.ai_x, layout.ai_y));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(inner_pos) = main_ww.inner_position() {
            let scale = main_ww.scale_factor().unwrap_or(1.0);
            let abs_x = inner_pos.x as f64 / scale + layout.ai_x;
            let abs_y = inner_pos.y as f64 / scale + layout.ai_y;
            let _ = ww.set_position(tauri::LogicalPosition::new(abs_x, abs_y));
        }
    }

    Ok(ww)
}

/// On Windows, call SetParent to make child window an OS-level child of main.
/// This makes the child window move together with the main window during drag.
/// We do NOT change the window style to WS_CHILD to avoid breaking WebView2 input.
#[cfg(target_os = "windows")]
fn set_parent_window(main_ww: &tauri::WebviewWindow, child_ww: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::SetParent;

    let main_hwnd = main_ww.hwnd().unwrap().0 as windows_sys::Win32::Foundation::HWND;
    let child_hwnd = child_ww.hwnd().unwrap().0 as windows_sys::Win32::Foundation::HWND;

    unsafe {
        SetParent(child_hwnd, main_hwnd);
    }
}

/// Switch visible service webview: show the active one, hide others.
pub fn switch_service(app_handle: &tauri::AppHandle, active_service_id: &str, state: &AppState) {
    let main_ww = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };

    let layout = match get_layout_params(&main_ww, state) {
        Some(l) => l,
        None => return,
    };

    for label in &state.created_webview_labels {
        if let Some(child_ww) = app_handle.get_webview_window(label) {
            let service_id = label.strip_prefix("service-").unwrap_or(label);
            if service_id == active_service_id {
                position_child(
                    &main_ww,
                    &child_ww,
                    layout.service_x,
                    layout.service_y,
                    layout.service_width,
                    layout.service_height,
                );
                let _ = child_ww.show();
            } else {
                let _ = child_ww.hide();
            }
        }
    }
}

/// Update layout of all child webview windows (services + AI).
pub fn update_layout(app_handle: &tauri::AppHandle, state: &AppState) {
    let main_ww = match app_handle.get_webview_window("main") {
        Some(w) => w,
        None => return,
    };

    let layout = match get_layout_params(&main_ww, state) {
        Some(l) => l,
        None => return,
    };

    // Update service webview windows
    for label in &state.created_webview_labels {
        if let Some(child_ww) = app_handle.get_webview_window(label) {
            let service_id = label.strip_prefix("service-").unwrap_or(label);
            if service_id == state.active_service_id {
                position_child(
                    &main_ww,
                    &child_ww,
                    layout.service_x,
                    layout.service_y,
                    layout.service_width,
                    layout.service_height,
                );
                let _ = child_ww.show();
            } else {
                let _ = child_ww.hide();
            }
        }
    }

    // Update AI webview window
    if state.ai_webview_created {
        if let Some(ai_ww) = app_handle.get_webview_window("ai-webview") {
            if state.show_ai_companion {
                position_child(
                    &main_ww,
                    &ai_ww,
                    layout.ai_x,
                    layout.ai_y,
                    layout.ai_width,
                    layout.ai_height,
                );
                let _ = ai_ww.show();
            } else {
                let _ = ai_ww.hide();
            }
        }
    }
}

/// Called when the main window is resized.
pub fn on_main_window_resized(app_handle: &tauri::AppHandle, state: &AppState) {
    update_layout(app_handle, state);
}
