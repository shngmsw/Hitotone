use crate::state::{AiService, AppState, Service};
use crate::store;
use crate::webview_manager;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::RwLock;

/// chrome WebView から modal 表示をリクエスト。service/AI WebView を隠してから親に通知。
#[tauri::command]
pub async fn request_open_modal(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    modal_type: String,
) -> Result<(), ()> {
    let active_service_id = state.read().await.active_service_id.clone();
    if !active_service_id.is_empty() {
        let label = format!("service-{}", active_service_id);
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.hide();
        }
    }
    if let Some(wv) = app.get_webview("ai-webview") {
        let _ = wv.hide();
    }
    let _ = app.emit("open-modal", &modal_type);
    Ok(())
}

// ========================================
// Service commands
// ========================================

#[tauri::command]
pub async fn get_services(state: State<'_, RwLock<AppState>>) -> Result<Vec<Service>, ()> {
    Ok(state.read().await.services.clone())
}

#[tauri::command]
pub async fn add_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service: Service,
) -> Result<Vec<Service>, ()> {
    let new_service = Service {
        id: format!("custom-{}", chrono_now()),
        name: service.name,
        url: service.url,
        icon: if service.icon.is_empty() {
            "\u{1F517}".to_string()
        } else {
            service.icon
        },
        enabled: true,
        favicon_url: None,
    };
    let services = {
        let mut s = state.write().await;
        s.services.push(new_service);
        store::save_services(&app, &s.services);
        s.services.clone()
    };
    let _ = app.emit("service-list-updated", ());
    Ok(services)
}

#[tauri::command]
pub async fn remove_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<Vec<Service>, ()> {
    let label = format!("service-{}", service_id);

    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.close();
    }

    let services = {
        let mut s = state.write().await;
        s.services.retain(|svc| svc.id != service_id);
        s.created_webview_labels.retain(|l| l != &label);
        store::save_services(&app, &s.services);
        s.services.clone()
    };
    let _ = app.emit("service-list-updated", ());
    Ok(services)
}

#[tauri::command]
pub async fn update_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service: Service,
) -> Result<Vec<Service>, ()> {
    let mut s = state.write().await;
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
    Ok(s.services.clone())
}

#[tauri::command]
pub async fn reorder_services(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    services: Vec<Service>,
) -> Result<Vec<Service>, ()> {
    let mut s = state.write().await;

    let mut current_ids: Vec<String> = s.services.iter().map(|svc| svc.id.clone()).collect();
    let mut new_ids: Vec<String> = services.iter().map(|svc| svc.id.clone()).collect();
    current_ids.sort();
    new_ids.sort();

    if current_ids != new_ids {
        return Ok(s.services.clone());
    }

    s.services = services;
    store::save_services(&app, &s.services);
    let result = s.services.clone();
    drop(s);
    let _ = app.emit("service-list-updated", ());
    Ok(result)
}

#[tauri::command]
pub async fn get_active_service(state: State<'_, RwLock<AppState>>) -> Result<String, ()> {
    Ok(state.read().await.active_service_id.clone())
}

#[tauri::command]
pub async fn set_active_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<String, ()> {
    state.write().await.active_service_id = service_id.clone();
    store::save_value(&app, "activeServiceId", &service_id);
    Ok(service_id)
}

// ========================================
// AI Service commands
// ========================================

#[tauri::command]
pub async fn get_ai_services(state: State<'_, RwLock<AppState>>) -> Result<Vec<AiService>, ()> {
    Ok(state.read().await.ai_services.clone())
}

#[tauri::command]
pub async fn get_active_ai_service(
    state: State<'_, RwLock<AppState>>,
) -> Result<Option<AiService>, ()> {
    let s = state.read().await;
    let found = s
        .ai_services
        .iter()
        .find(|svc| svc.id == s.active_ai_service_id)
        .cloned()
        .or_else(|| s.ai_services.first().cloned());
    Ok(found)
}

#[tauri::command]
pub async fn set_active_ai_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<Option<AiService>, ()> {
    let mut s = state.write().await;
    let service = s
        .ai_services
        .iter()
        .find(|svc| svc.id == service_id)
        .cloned();
    if service.is_some() {
        s.active_ai_service_id = service_id.clone();
        store::save_value(&app, "activeAiServiceId", &service_id);
    }
    Ok(service)
}

#[tauri::command]
pub async fn add_ai_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service: AiService,
) -> Result<Vec<AiService>, ()> {
    let mut s = state.write().await;
    let new_service = AiService {
        id: format!("ai-{}", chrono_now()),
        name: service.name,
        url: service.url,
        is_default: false,
    };
    s.ai_services.push(new_service);
    store::save_ai_services(&app, &s.ai_services);
    Ok(s.ai_services.clone())
}

#[tauri::command]
pub async fn remove_ai_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<Vec<AiService>, ()> {
    let mut s = state.write().await;

    let is_default = s
        .ai_services
        .iter()
        .find(|svc| svc.id == service_id)
        .is_some_and(|svc| svc.is_default);

    if is_default {
        return Ok(s.ai_services.clone());
    }

    s.ai_services.retain(|svc| svc.id != service_id);

    if s.active_ai_service_id == service_id {
        if let Some(first) = s.ai_services.first() {
            s.active_ai_service_id = first.id.clone();
            store::save_value(&app, "activeAiServiceId", &s.active_ai_service_id);
        }
    }

    store::save_ai_services(&app, &s.ai_services);
    Ok(s.ai_services.clone())
}

// ========================================
// AI Companion settings
// ========================================

#[tauri::command]
pub async fn get_show_ai_companion(state: State<'_, RwLock<AppState>>) -> Result<bool, ()> {
    Ok(state.read().await.show_ai_companion)
}

#[tauri::command]
pub async fn set_show_ai_companion(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    show: bool,
) -> Result<bool, ()> {
    state.write().await.show_ai_companion = show;
    store::save_value(&app, "showAiCompanion", &show);
    Ok(show)
}

#[tauri::command]
pub async fn get_ai_width(state: State<'_, RwLock<AppState>>) -> Result<u32, ()> {
    Ok(state.read().await.ai_width)
}

#[tauri::command]
pub async fn set_ai_width(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    width: u32,
) -> Result<u32, ()> {
    let valid_width = width.clamp(300, 800);
    state.write().await.ai_width = valid_width;
    store::save_value(&app, "aiWidth", &valid_width);
    Ok(valid_width)
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
pub fn window_start_drag(app: AppHandle) {
    if let Some(ww) = app.get_webview_window("main") {
        let _ = ww.start_dragging();
    }
}

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
pub async fn create_service_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
    url: String,
) -> Result<(), String> {
    let label = format!("service-{}", service_id);

    if let Some(wv) = app.get_webview(&label) {
        if let Ok(parsed_url) = url.parse::<url::Url>() {
            let _ = wv.navigate(parsed_url);
        }
        let mut s = state.write().await;
        if !s.created_webview_labels.contains(&label) {
            s.created_webview_labels.push(label);
        }
        return Ok(());
    }

    let rect = {
        let s = state.read().await;
        app.get_window("main")
            .and_then(|w| webview_manager::get_service_zone_rect(&w, &s))
            .ok_or_else(|| "Failed to compute service zone rect".to_string())?
    };

    crate::webview_ops::create_service(&app, label.clone(), url, rect).await?;

    let mut s = state.write().await;
    if !s.created_webview_labels.contains(&label) {
        s.created_webview_labels.push(label);
    }
    Ok(())
}

#[tauri::command]
pub async fn create_all_service_webviews(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<(), String> {
    let services_to_register = {
        let s = state.read().await;
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
        if app.get_webview(&label).is_some() {
            registered_labels.push(label);
        } else {
            println!(
                "[create_all_service_webviews] Webview {} not found (not created in setup)",
                label
            );
        }
    }

    let mut s = state.write().await;

    if app.get_webview("ai-webview").is_some() {
        s.ai_webview_created = true;
    }

    for label in registered_labels {
        if !s.created_webview_labels.contains(&label) {
            s.created_webview_labels.push(label);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn switch_service_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<(), String> {
    println!("[commands] switch_service_webview: {}", service_id);
    let label = format!("service-{}", service_id);

    if app.get_webview(&label).is_none() {
        let url_opt = {
            let s = state.read().await;
            s.services
                .iter()
                .find(|svc| svc.id == service_id)
                .map(|svc| svc.url.clone())
        };
        if let Some(url) = url_opt {
            let rect = {
                let s = state.read().await;
                app.get_window("main")
                    .and_then(|w| webview_manager::get_service_zone_rect(&w, &s))
            };
            if let Some(r) = rect {
                crate::webview_ops::create_service(&app, label.clone(), url, r).await?;
                let mut s = state.write().await;
                if !s.created_webview_labels.contains(&label) {
                    s.created_webview_labels.push(label);
                }
            }
        }
    }

    let snapshot = state.read().await.clone();
    webview_manager::switch_service(&app, &service_id, &snapshot);
    Ok(())
}

#[tauri::command]
pub async fn remove_service_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<(), String> {
    let label = format!("service-{}", service_id);

    if let Some(wv) = app.get_webview(&label) {
        let _ = wv.close();
    }

    let mut s = state.write().await;
    s.created_webview_labels.retain(|l| l != &label);
    Ok(())
}

#[tauri::command]
pub async fn hide_all_child_webviews(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<(), ()> {
    let active_service_id = state.read().await.active_service_id.clone();

    if !active_service_id.is_empty() {
        let label = format!("service-{}", active_service_id);
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.hide();
        }
    }

    if let Some(wv) = app.get_webview("ai-webview") {
        let _ = wv.hide();
    }
    Ok(())
}

#[tauri::command]
pub async fn restore_child_webviews(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<(), ()> {
    let (active_service_id, show_ai_companion) = {
        let s = state.read().await;
        (s.active_service_id.clone(), s.show_ai_companion)
    };

    if !active_service_id.is_empty() {
        let label = format!("service-{}", active_service_id);
        if let Some(wv) = app.get_webview(&label) {
            let _ = wv.show();
        }
    }

    if show_ai_companion {
        if let Some(wv) = app.get_webview("ai-webview") {
            let _ = wv.show();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn create_ai_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    url: String,
) -> Result<(), String> {
    if let Some(wv) = app.get_webview("ai-webview") {
        if let Ok(parsed_url) = url.parse::<url::Url>() {
            let _ = wv.navigate(parsed_url);
        }
        state.write().await.ai_webview_created = true;
        return Ok(());
    }

    let rect = {
        let s = state.read().await;
        if let Some(main_win) = app.get_window("main") {
            let vp =
                crate::webview_manager::get_viewport(&main_win).ok_or("Failed to get viewport")?;
            let tree = crate::layout::build_tree_from_state(&s);
            let rects = crate::layout::compute_rects(&tree, vp);
            rects
                .into_iter()
                .find(|(id, _)| id.0 == "ai")
                .map(|(_, r)| r)
                .ok_or_else(|| "ai pane not found in layout".to_string())?
        } else {
            return Err("Main window not found".into());
        }
    };

    let gap = crate::layout::RESIZE_GAP;
    let ai_rect = crate::layout::Rect {
        x: rect.x + gap,
        y: rect.y,
        width: (rect.width - gap).max(0.0),
        height: rect.height,
    };
    crate::webview_ops::create_ai(&app, url, ai_rect).await?;
    state.write().await.ai_webview_created = true;
    Ok(())
}

#[tauri::command]
pub async fn setup_ai_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    url: String,
    _width: u32,
) -> Result<(), String> {
    create_ai_webview(app, state, url).await
}

#[tauri::command]
pub async fn toggle_ai_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<bool, String> {
    let show = {
        let mut s = state.write().await;
        s.show_ai_companion = !s.show_ai_companion;
        s.show_ai_companion
    };
    store::save_value(&app, "showAiCompanion", &show);

    println!("[commands] toggle_ai_webview: show={}", show);
    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);

    Ok(show)
}

#[tauri::command]
pub async fn resize_ai_webview(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    width: u32,
) -> Result<(), String> {
    let valid_width = width.clamp(300, 800);
    state.write().await.ai_width = valid_width;
    store::save_value(&app, "aiWidth", &valid_width);

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    Ok(())
}

#[tauri::command]
pub async fn update_layout(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
) -> Result<(), String> {
    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    Ok(())
}

#[tauri::command]
pub async fn switch_ai_service(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
) -> Result<Option<AiService>, String> {
    let service = {
        let mut s = state.write().await;
        let found = s
            .ai_services
            .iter()
            .find(|svc| svc.id == service_id)
            .cloned();
        if found.is_some() {
            s.active_ai_service_id = service_id.clone();
        }
        found
    };

    if let Some(ref svc) = service {
        store::save_value(&app, "activeAiServiceId", &service_id);
        if let Some(ai_wv) = app.get_webview("ai-webview") {
            let url: url::Url = svc
                .url
                .parse()
                .map_err(|e: url::ParseError| e.to_string())?;
            let _ = ai_wv.navigate(url);
        }
    }

    Ok(service)
}

#[tauri::command]
pub async fn send_to_ai_webview(app: AppHandle, text: String) -> Result<(), String> {
    if let Some(ai_wv) = app.get_webview("ai-webview") {
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
        ai_wv.eval(&js).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ========================================
// Popup window
// ========================================

pub fn open_popup_window_internal(
    app: &tauri::AppHandle,
    url: String,
    source_label: Option<String>,
    target_domain: Option<String>,
) {
    println!("[open_popup_window] Opening popup: {}", url);

    let parsed_url = match url.parse::<url::Url>() {
        Ok(u) => u,
        Err(e) => {
            println!("[open_popup_window] URL parse error: {}", e);
            return;
        }
    };

    if let Some(existing) = app.get_webview_window("popup-auth") {
        let _ = existing.close();
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
    .user_agent(crate::webview_manager::chrome_user_agent())
    .initialization_script(crate::webview_manager::browser_spoof_script())
    .on_navigation(move |nav_url| {
        let url_str = nav_url.as_str();
        println!("[popup-auth] navigating to: {}", url_str);

        let mut is_auth_complete = false;

        if let Some(ref domain) = target_domain {
            if url_str.contains(domain) && !crate::webview_manager::is_auth_url(url_str) {
                is_auth_complete = true;
            }
        } else {
            is_auth_complete = (url_str.contains("app.slack.com")
                || (url_str.contains(".slack.com") && !url_str.contains("accounts.google")))
                && !url_str.contains("/signin")
                && !url_str.contains("/oauth")
                && !url_str.contains("/sso")
                && !url_str.contains("oauth2.slack.com");
        }

        if is_auth_complete {
            println!("[popup-auth] Auth completed! returning to main webview");
            let owned_url = url_str.to_string();
            let app2 = app_for_nav.clone();
            let pop_source_label = source_label.clone();

            tokio::spawn(async move {
                if let Some(label) = pop_source_label {
                    if let Some(wv) = app2.get_webview(&label) {
                        if let Ok(u) = owned_url.parse::<url::Url>() {
                            let _ = wv.navigate(u);
                        }
                        let _ = wv.show();
                    }
                } else {
                    let slack_label = {
                        let state = app2.state::<RwLock<crate::state::AppState>>();
                        let s = state.read().await;
                        s.services
                            .iter()
                            .find(|svc| svc.url.contains("slack.com"))
                            .map(|svc| format!("service-{}", svc.id))
                    };
                    if let Some(label) = slack_label {
                        if let Some(slack_wv) = app2.get_webview(&label) {
                            if let Ok(u) = owned_url.parse::<url::Url>() {
                                let _ = slack_wv.navigate(u);
                            }
                            let _ = slack_wv.show();
                        }
                    }
                }

                let _ = app2.emit("auth-completed", &owned_url);
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
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
    open_popup_window_internal(&app, url, None, None);
    Ok(())
}

// ========================================
// Notification
// ========================================

#[tauri::command]
pub async fn update_notification_count(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
    count: u32,
) -> Result<(), String> {
    state
        .write()
        .await
        .badge_counts
        .insert(service_id.clone(), count);

    let _ = app.emit(
        "badge-updated",
        serde_json::json!({
            "serviceId": service_id,
            "count": count
        }),
    );

    Ok(())
}

#[tauri::command]
pub async fn update_favicon(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    service_id: String,
    favicon_url: String,
) -> Result<(), String> {
    {
        let mut s = state.write().await;
        if let Some(svc) = s.services.iter_mut().find(|svc| svc.id == service_id) {
            if svc.favicon_url.as_deref() == Some(&favicon_url) {
                return Ok(());
            }
            svc.favicon_url = Some(favicon_url.clone());
        }
        store::save_services(&app, &s.services);
    }

    let _ = app.emit(
        "favicon-updated",
        serde_json::json!({
            "serviceId": service_id,
            "faviconUrl": favicon_url
        }),
    );

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
