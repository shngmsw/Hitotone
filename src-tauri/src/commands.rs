use crate::state::{AiService, AppState, Service};
use crate::store;
use crate::webview_manager;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

// ========================================
// Service commands
// ========================================

#[tauri::command]
pub fn get_services(state: State<Mutex<AppState>>) -> Vec<Service> {
    let s = state.lock().unwrap();
    s.services.clone()
}

#[tauri::command]
pub fn add_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service: Service,
) -> Vec<Service> {
    let mut s = state.lock().unwrap();
    let new_service = Service {
        id: format!("custom-{}", chrono_now()),
        name: service.name,
        url: service.url,
        icon: if service.icon.is_empty() {
            "\u{1F517}".to_string() // 🔗
        } else {
            service.icon
        },
        enabled: true,
        favicon_url: None,
    };
    s.services.push(new_service);
    store::save_services(&app, &s.services);
    s.services.clone()
}

#[tauri::command]
pub fn remove_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Vec<Service> {
    let label = format!("service-{}", service_id);

    // Close the webview window
    if let Some(ww) = app.get_webview_window(&label) {
        let _ = ww.close();
    }

    let mut s = state.lock().unwrap();
    s.services.retain(|svc| svc.id != service_id);
    s.created_webview_labels.retain(|l| l != &label);

    store::save_services(&app, &s.services);
    s.services.clone()
}

#[tauri::command]
pub fn update_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service: Service,
) -> Vec<Service> {
    let mut s = state.lock().unwrap();
    if let Some(existing) = s.services.iter_mut().find(|svc| svc.id == service.id) {
        existing.name = service.name;
        existing.url = service.url;
        existing.icon = service.icon;
        existing.enabled = service.enabled;
        if service.favicon_url.is_some() {
            existing.favicon_url = service.favicon_url;
        }
    }
    store::save_services(&app, &s.services);
    s.services.clone()
}

#[tauri::command]
pub fn reorder_services(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    services: Vec<Service>,
) -> Vec<Service> {
    let mut s = state.lock().unwrap();

    let mut current_ids: Vec<String> = s.services.iter().map(|svc| svc.id.clone()).collect();
    let mut new_ids: Vec<String> = services.iter().map(|svc| svc.id.clone()).collect();
    current_ids.sort();
    new_ids.sort();

    if current_ids != new_ids {
        return s.services.clone();
    }

    s.services = services;
    store::save_services(&app, &s.services);
    s.services.clone()
}

#[tauri::command]
pub fn get_active_service(state: State<Mutex<AppState>>) -> String {
    let s = state.lock().unwrap();
    s.active_service_id.clone()
}

#[tauri::command]
pub fn set_active_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> String {
    let mut s = state.lock().unwrap();
    s.active_service_id = service_id.clone();
    store::save_value(&app, "activeServiceId", &service_id);
    service_id
}

// ========================================
// AI Service commands
// ========================================

#[tauri::command]
pub fn get_ai_services(state: State<Mutex<AppState>>) -> Vec<AiService> {
    let s = state.lock().unwrap();
    s.ai_services.clone()
}

#[tauri::command]
pub fn get_active_ai_service(state: State<Mutex<AppState>>) -> Option<AiService> {
    let s = state.lock().unwrap();
    s.ai_services
        .iter()
        .find(|svc| svc.id == s.active_ai_service_id)
        .cloned()
        .or_else(|| s.ai_services.first().cloned())
}

#[tauri::command]
pub fn set_active_ai_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Option<AiService> {
    let mut s = state.lock().unwrap();
    let service = s.ai_services.iter().find(|svc| svc.id == service_id).cloned();
    if service.is_some() {
        s.active_ai_service_id = service_id.clone();
        store::save_value(&app, "activeAiServiceId", &service_id);
    }
    service
}

#[tauri::command]
pub fn add_ai_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service: AiService,
) -> Vec<AiService> {
    let mut s = state.lock().unwrap();
    let new_service = AiService {
        id: format!("ai-{}", chrono_now()),
        name: service.name,
        url: service.url,
        is_default: false,
    };
    s.ai_services.push(new_service);
    store::save_ai_services(&app, &s.ai_services);
    s.ai_services.clone()
}

#[tauri::command]
pub fn remove_ai_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Vec<AiService> {
    let mut s = state.lock().unwrap();

    let is_default = s
        .ai_services
        .iter()
        .find(|svc| svc.id == service_id)
        .map_or(false, |svc| svc.is_default);

    if is_default {
        return s.ai_services.clone();
    }

    s.ai_services.retain(|svc| svc.id != service_id);

    if s.active_ai_service_id == service_id {
        if let Some(first) = s.ai_services.first() {
            s.active_ai_service_id = first.id.clone();
            store::save_value(&app, "activeAiServiceId", &s.active_ai_service_id);
        }
    }

    store::save_ai_services(&app, &s.ai_services);
    s.ai_services.clone()
}

// ========================================
// AI Companion settings
// ========================================

#[tauri::command]
pub fn get_show_ai_companion(state: State<Mutex<AppState>>) -> bool {
    let s = state.lock().unwrap();
    s.show_ai_companion
}

#[tauri::command]
pub fn set_show_ai_companion(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    show: bool,
) -> bool {
    let mut s = state.lock().unwrap();
    s.show_ai_companion = show;
    store::save_value(&app, "showAiCompanion", &show);
    show
}

#[tauri::command]
pub fn get_ai_width(state: State<Mutex<AppState>>) -> u32 {
    let s = state.lock().unwrap();
    s.ai_width
}

#[tauri::command]
pub fn set_ai_width(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    width: u32,
) -> u32 {
    let mut s = state.lock().unwrap();
    let valid_width = width.max(300).min(800);
    s.ai_width = valid_width;
    store::save_value(&app, "aiWidth", &valid_width);
    valid_width
}

// ========================================
// Platform info
// ========================================

#[tauri::command]
pub fn get_platform() -> String {
    if cfg!(target_os = "macos") {
        "darwin".to_string()
    } else if cfg!(target_os = "windows") {
        "win32".to_string()
    } else {
        "linux".to_string()
    }
}

// ========================================
// Window controls
// ========================================

#[tauri::command]
pub fn window_minimize(app: AppHandle) {
    if let Some(ww) = app.get_webview_window("main") {
        let _ = ww.minimize();
    }
}

#[tauri::command]
pub fn window_maximize(app: AppHandle) {
    if let Some(ww) = app.get_webview_window("main") {
        if ww.is_maximized().unwrap_or(false) {
            let _ = ww.unmaximize();
        } else {
            let _ = ww.maximize();
        }
    }
}

#[tauri::command]
pub fn window_close(app: AppHandle) {
    if let Some(ww) = app.get_webview_window("main") {
        let _ = ww.close();
    }
}

#[tauri::command]
pub fn window_is_maximized(app: AppHandle) -> bool {
    app.get_webview_window("main")
        .map(|w| w.is_maximized().unwrap_or(false))
        .unwrap_or(false)
}

// ========================================
// WebView management commands
// ========================================

#[tauri::command]
pub fn create_service_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
    url: String,
) -> Result<(), String> {
    let label = format!("service-{}", service_id);

    // If webview window already exists (created in setup), just navigate it
    if let Some(ww) = app.get_webview_window(&label) {
        if let Ok(parsed_url) = url.parse::<url::Url>() {
            let _ = ww.navigate(parsed_url);
        }
        let mut s = state.lock().unwrap();
        if !s.created_webview_labels.contains(&label) {
            s.created_webview_labels.push(label);
        }
        return Ok(());
    }

    // Window doesn't exist yet (dynamically added service)
    // Cannot create WebviewWindow from command thread (deadlocks on Windows)
    // Instead, emit an event so the app can be restarted or handle it gracefully
    println!("[create_service_webview] Window {} not found - dynamic creation not supported, requires app restart", label);
    Err(format!("Service webview '{}' was not created at startup. Please restart the app to load new services.", label))
}

#[tauri::command]
pub fn create_all_service_webviews(
    app: AppHandle,
    state: State<Mutex<AppState>>,
) -> Result<(), String> {
    // Windows are already created in setup() on the main thread.
    // This command just discovers and registers them.
    let services_to_register = {
        let s = state.lock().unwrap();
        s.services
            .iter()
            .filter(|svc| svc.enabled)
            .map(|svc| svc.id.clone())
            .collect::<Vec<_>>()
    };

    println!(
        "[create_all_service_webviews] Registering {} service webviews (already created in setup)",
        services_to_register.len()
    );

    let mut registered_labels: Vec<String> = Vec::new();
    for service_id in &services_to_register {
        let label = format!("service-{}", service_id);
        if app.get_webview_window(&label).is_some() {
            registered_labels.push(label);
        } else {
            println!("[create_all_service_webviews] Window {} not found (not created in setup)", label);
        }
    }

    // Register AI webview
    if app.get_webview_window("ai-webview").is_some() {
        let mut s = state.lock().unwrap();
        s.ai_webview_created = true;
    }

    // Update state with registered labels
    {
        let mut s = state.lock().unwrap();
        for label in registered_labels {
            if !s.created_webview_labels.contains(&label) {
                s.created_webview_labels.push(label);
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn switch_service_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Result<(), String> {
    let s = state.lock().unwrap();
    webview_manager::switch_service(&app, &service_id, &s);
    Ok(())
}

#[tauri::command]
pub fn remove_service_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Result<(), String> {
    let label = format!("service-{}", service_id);

    if let Some(ww) = app.get_webview_window(&label) {
        let _ = ww.close();
    }

    let mut s = state.lock().unwrap();
    s.created_webview_labels.retain(|l| l != &label);
    Ok(())
}

#[tauri::command]
pub fn create_ai_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    url: String,
) -> Result<(), String> {
    // AI webview should already be created in setup()
    if let Some(ww) = app.get_webview_window("ai-webview") {
        if let Ok(parsed_url) = url.parse::<url::Url>() {
            let _ = ww.navigate(parsed_url);
        }
        let mut s = state.lock().unwrap();
        s.ai_webview_created = true;
        return Ok(());
    }

    // Cannot create from command thread (deadlocks on Windows)
    println!("[create_ai_webview] AI webview not found - requires app restart");
    Err("AI webview was not created at startup. Please restart the app.".to_string())
}

#[tauri::command]
pub fn setup_ai_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    url: String,
    _width: u32,
) -> Result<(), String> {
    create_ai_webview(app, state, url)
}

#[tauri::command]
pub fn toggle_ai_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
) -> Result<bool, String> {
    let mut s = state.lock().unwrap();
    s.show_ai_companion = !s.show_ai_companion;
    let show = s.show_ai_companion;
    store::save_value(&app, "showAiCompanion", &show);

    webview_manager::update_layout(&app, &s);
    Ok(show)
}

#[tauri::command]
pub fn resize_ai_webview(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    width: u32,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    let valid_width = width.max(300).min(800);
    s.ai_width = valid_width;
    store::save_value(&app, "aiWidth", &valid_width);

    webview_manager::update_layout(&app, &s);
    Ok(())
}

#[tauri::command]
pub fn update_layout(
    app: AppHandle,
    state: State<Mutex<AppState>>,
) -> Result<(), String> {
    let s = state.lock().unwrap();
    webview_manager::update_layout(&app, &s);
    Ok(())
}

#[tauri::command]
pub fn switch_ai_service(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
) -> Result<Option<AiService>, String> {
    let mut s = state.lock().unwrap();
    let service = s.ai_services.iter().find(|svc| svc.id == service_id).cloned();

    if let Some(ref svc) = service {
        s.active_ai_service_id = service_id.clone();
        store::save_value(&app, "activeAiServiceId", &service_id);

        if let Some(ai_ww) = app.get_webview_window("ai-webview") {
            let url: url::Url = svc.url.parse().map_err(|e: url::ParseError| e.to_string())?;
            let _ = ai_ww.navigate(url);
        }
    }

    Ok(service)
}

#[tauri::command]
pub fn send_to_ai_webview(app: AppHandle, text: String) -> Result<(), String> {
    if let Some(ai_ww) = app.get_webview_window("ai-webview") {
        let js = format!(
            r#"(function() {{
                const textareas = document.querySelectorAll('textarea, [contenteditable="true"], .ql-editor, [role="textbox"]');
                for (const el of textareas) {{
                    if (el.offsetParent !== null) {{
                        if (el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') {{
                            el.value = {};
                            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        }} else {{
                            el.textContent = {};
                            el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        }}
                        el.focus();
                        return;
                    }}
                }}
                navigator.clipboard.writeText({});
            }})();"#,
            serde_json::to_string(&text).unwrap_or_default(),
            serde_json::to_string(&text).unwrap_or_default(),
            serde_json::to_string(&text).unwrap_or_default(),
        );
        ai_ww.eval(&js).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ========================================
// Popup window
// ========================================

/// ポップアップウィンドウを開く内部関数（webview_manager.rsからも呼び出し可能）
pub fn open_popup_window_internal(app: &tauri::AppHandle, url: String) {
    println!("[open_popup_window] Opening popup: {}", url);

    let parsed_url = match url.parse::<url::Url>() {
        Ok(u) => u,
        Err(e) => {
            println!("[open_popup_window] URL parse error: {}", e);
            return;
        }
    };

    // 既存のポップアップウィンドウがあれば閉じる
    if let Some(existing) = app.get_webview_window("popup-auth") {
        let _ = existing.close();
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    let app_for_nav = app.clone();

    let result = tauri::WebviewWindowBuilder::new(
        app,
        "popup-auth",
        tauri::WebviewUrl::External(parsed_url),
    )
    .title("認証")
    .inner_size(520.0, 750.0)
    .center()
    .decorations(true)
    .on_navigation(move |nav_url| {
        let url_str = nav_url.as_str();
        println!("[popup-auth] navigating to: {}", url_str);

        // Slackワークスペースへの到達を検知して認証完了とみなす
        // app.slack.com または *.slack.com (signin/oauth以外)
        let is_slack_workspace = (url_str.contains("app.slack.com")
            || (url_str.contains(".slack.com") && !url_str.contains("accounts.google")))
            && !url_str.contains("/signin")
            && !url_str.contains("/oauth")
            && !url_str.contains("/sso")
            && !url_str.contains("oauth2.slack.com");

        if is_slack_workspace {
            println!("[popup-auth] workspace reached! navigating main slack webview");
            let owned_url = url_str.to_string();
            let app2 = app_for_nav.clone();
            std::thread::spawn(move || {
                // AppStateからSlack系のWebViewを探してナビゲート
                let slack_label = {
                    let state = app2.state::<std::sync::Mutex<AppState>>();
                    let s = state.lock().unwrap();
                    s.services
                        .iter()
                        .find(|svc| svc.url.contains("slack.com"))
                        .map(|svc| format!("service-{}", svc.id))
                };
                if let Some(label) = slack_label {
                    if let Some(slack_ww) = app2.get_webview_window(&label) {
                        if let Ok(u) = owned_url.parse::<url::Url>() {
                            let _ = slack_ww.navigate(u);
                        }
                        let _ = slack_ww.show();
                    }
                }
                let _ = app2.emit("auth-completed", &owned_url);
                std::thread::sleep(std::time::Duration::from_millis(1000));
                if let Some(w) = app2.get_webview_window("popup-auth") {
                    let _ = w.close();
                }
            });
        }
        true
    })
    .build();

    match result {
        Ok(ref _ww) => {
            println!("[open_popup_window] Popup opened successfully");
            #[cfg(debug_assertions)]
            _ww.open_devtools();
        }
        Err(ref e) => println!("[open_popup_window] Failed to open popup: {}", e),
    }
}

#[tauri::command]
pub fn open_popup_window(app: AppHandle, url: String) -> Result<(), String> {
    open_popup_window_internal(&app, url);
    Ok(())
}

// ========================================
// Notification
// ========================================

#[tauri::command]
pub fn update_notification_count(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
    count: u32,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();
    s.badge_counts.insert(service_id.clone(), count);

    let _ = app.emit("badge-updated", serde_json::json!({
        "serviceId": service_id,
        "count": count
    }));

    Ok(())
}

#[tauri::command]
pub fn update_favicon(
    app: AppHandle,
    state: State<Mutex<AppState>>,
    service_id: String,
    favicon_url: String,
) -> Result<(), String> {
    let mut s = state.lock().unwrap();

    if let Some(svc) = s.services.iter_mut().find(|svc| svc.id == service_id) {
        if svc.favicon_url.as_deref() == Some(&favicon_url) {
            return Ok(());
        }
        svc.favicon_url = Some(favicon_url.clone());
    }
    store::save_services(&app, &s.services);

    let _ = app.emit("favicon-updated", serde_json::json!({
        "serviceId": service_id,
        "faviconUrl": favicon_url
    }));

    Ok(())
}

// ========================================
// Helpers
// ========================================

fn chrono_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
