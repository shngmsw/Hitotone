use crate::layout::Rect;
use tauri::{AppHandle, Manager};

/// Run `f` on the main thread via oneshot dispatch.
/// Do NOT call this from setup() (already on main thread) — use dispatch_main_inline instead.
pub async fn dispatch_main<F, R>(app: &AppHandle, f: F) -> Result<R, String>
where
    F: FnOnce(AppHandle) -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel::<R>();
    let app_clone = app.clone();
    app.run_on_main_thread(move || {
        let result = f(app_clone);
        let _ = tx.send(result);
    })
    .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

/// Run `f` inline on the current thread (for use in setup hook, already on main thread).
#[allow(dead_code)]
pub fn dispatch_main_inline<F, R>(app: &AppHandle, f: F) -> R
where
    F: FnOnce(&AppHandle) -> R,
{
    f(app)
}

// ---- WebView operation wrappers ----

#[allow(dead_code)]
pub async fn position(app: &AppHandle, label: String, rect: Rect) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.set_position(tauri::LogicalPosition::new(rect.x as f64, rect.y as f64));
            let _ = wv.set_size(tauri::LogicalSize::new(
                rect.width as f64,
                rect.height as f64,
            ));
        }
    })
    .await
}

#[allow(dead_code)]
pub async fn show(app: &AppHandle, label: String) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.show();
        }
    })
    .await
}

#[allow(dead_code)]
pub async fn hide(app: &AppHandle, label: String) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.hide();
        }
    })
    .await
}

#[allow(dead_code)]
pub async fn navigate(app: &AppHandle, label: String, url: url::Url) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.navigate(url);
        }
    })
    .await
}

#[allow(dead_code)]
pub async fn close(app: &AppHandle, label: String) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.close();
        }
    })
    .await
}

#[allow(dead_code)]
pub async fn eval(app: &AppHandle, label: String, js: String) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.eval(&js);
        }
    })
    .await
}

// ---- WebView creation wrappers ----

#[allow(dead_code)]
pub async fn create_chrome(app: &AppHandle, rect: Rect) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(main_win) = app.get_window("main") {
            crate::webview_manager::create_chrome_webview(&main_win, rect)
                .map(|_| ())
                .unwrap_or_else(|e| println!("[webview_ops] create_chrome error: {}", e));
        }
    })
    .await
}

pub async fn create_service(
    app: &AppHandle,
    label: String,
    url: String,
    rect: Rect,
) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(main_win) = app.get_window("main") {
            crate::webview_manager::create_service_webview(&app, &main_win, &label, &url, rect)
                .map(|_| ())
                .unwrap_or_else(|e| println!("[webview_ops] create_service error: {}", e));
        }
    })
    .await
}

pub async fn create_ai(app: &AppHandle, url: String, rect: Rect) -> Result<(), String> {
    dispatch_main(app, move |app| {
        if let Some(main_win) = app.get_window("main") {
            crate::webview_manager::create_ai_webview(&app, &main_win, &url, rect)
                .map(|_| ())
                .unwrap_or_else(|e| println!("[webview_ops] create_ai error: {}", e));
        }
    })
    .await
}

// ---- Inline creation for setup hook (already on main thread) ----

pub fn create_chrome_inline(app: &AppHandle, rect: Rect) -> Result<(), String> {
    let main_win = app.get_window("main").ok_or("no main window")?;
    crate::webview_manager::create_chrome_webview(&main_win, rect).map(|_| ())
}

pub fn create_service_inline(
    app: &AppHandle,
    label: &str,
    url: &str,
    rect: Rect,
) -> Result<tauri::Webview, String> {
    let main_win = app.get_window("main").ok_or("no main window")?;
    crate::webview_manager::create_service_webview(app, &main_win, label, url, rect)
}
