use crate::layout;
use crate::state::{
    AiService, AppState, LayoutNode, LayoutPreset, Pane, PaneId, PaneKind, Service, Space,
    SplitDirection, SplitSize,
};
use crate::store;
use crate::webview_manager;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::RwLock;
use uuid::Uuid;

/// chrome WebView から modal 表示をリクエスト。renderer 側の showModal() が hide_all_child_webviews を呼ぶ。
#[tauri::command]
pub async fn request_open_modal(app: AppHandle, modal_type: String) -> Result<(), ()> {
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
        // Remove service from all spaces' trees
        for space in s.spaces.iter_mut() {
            space.tree = layout::remove_service_from_tree(&space.tree, &service_id);
        }
        s.load_active_space_tree();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
        s.services.clone()
    };
    let _ = app.emit("service-list-updated", ());

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

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
            .and_then(|w| webview_manager::get_viewport(&w))
            .map(|vp| layout::compute_service_zone_rect(vp, &s))
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
    let label = format!("service-{}", service_id);

    // Create webview lazily if not yet created
    if app.get_webview(&label).is_none() {
        let (url_opt, zone_opt) = {
            let s = state.read().await;
            let url = s
                .services
                .iter()
                .find(|svc| svc.id == service_id)
                .map(|svc| svc.url.clone());
            let zone = app
                .get_window("main")
                .and_then(|w| webview_manager::get_viewport(&w))
                .map(|vp| layout::compute_service_zone_rect(vp, &s));
            (url, zone)
        };
        if let (Some(url), Some(zone)) = (url_opt, zone_opt) {
            crate::webview_ops::create_service(&app, label.clone(), url, zone).await?;
            let mut s = state.write().await;
            if !s.created_webview_labels.contains(&label) {
                s.created_webview_labels.push(label);
            }
        }
    }

    // Assign service to focused pane in tree, update active_service_id
    {
        let mut s = state.write().await;
        let focused = s.focused_pane_id.clone().unwrap_or(PaneId("root".into()));
        if let Ok(new_tree) =
            layout::assign_service_to_pane(&s.service_tree, &focused, service_id.clone())
        {
            s.service_tree = Arc::new(new_tree);
        }
        s.active_service_id = service_id.clone();
        s.sync_tree_to_active_space();
    }
    store::save_value(&app, "activeServiceId", &service_id);
    {
        let s = state.read().await;
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());
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
    let labels = state.read().await.created_webview_labels.clone();
    for label in &labels {
        if let Some(wv) = app.get_webview(label) {
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
    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
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
// Phase 3/4 — Pane tree commands
// ========================================

/// Returned by get_service_tree, apply_layout_preset, switch_service_in_pane.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneTreeState {
    pub tree: LayoutNode,
    pub focused_pane_id: Option<String>,
}

#[tauri::command]
pub async fn get_service_tree(state: State<'_, RwLock<AppState>>) -> Result<PaneTreeState, String> {
    let s = state.read().await;
    Ok(PaneTreeState {
        tree: (*s.service_tree).clone(),
        focused_pane_id: s.focused_pane_id.as_ref().map(|id| id.0.clone()),
    })
}

#[tauri::command]
pub async fn split_pane(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    pane_id: String,
    direction: SplitDirection,
    new_service_id: String,
) -> Result<LayoutNode, String> {
    let target = PaneId(pane_id);
    let new_pane_id = PaneId(Uuid::new_v4().to_string());
    let new_pane = Pane {
        id: new_pane_id.clone(),
        kind: PaneKind::Service(new_service_id.clone()),
        webview_label: format!("service-{}", new_service_id),
        visible: true,
    };
    let label = format!("service-{}", new_service_id);

    let (new_tree, url_opt, service_zone) = {
        let s = state.read().await;
        if layout::collect_service_ids(&s.service_tree).contains(&new_service_id) {
            return Err(format!(
                "service '{}' is already open in another pane",
                new_service_id
            ));
        }
        let new_tree = layout::split_pane_in_tree(&s.service_tree, &target, direction, new_pane)?;
        let url_opt = s
            .services
            .iter()
            .find(|svc| svc.id == new_service_id)
            .map(|svc| svc.url.clone());
        let zone = app
            .get_window("main")
            .and_then(|w| webview_manager::get_viewport(&w))
            .map(|vp| layout::compute_service_zone_rect(vp, &s));
        (new_tree, url_opt, zone)
    };

    {
        let mut s = state.write().await;
        s.service_tree = Arc::new(new_tree);
        s.focused_pane_id = Some(new_pane_id.clone());
        s.sync_tree_to_active_space();
    }

    if app.get_webview(&label).is_none() {
        if let (Some(url), Some(zone)) = (url_opt, service_zone) {
            let tree_arc = state.read().await.service_tree.clone();
            let rects = layout::compute_rects(&tree_arc, zone);
            let rect = rects
                .iter()
                .find(|(id, _)| id == &new_pane_id)
                .map(|(_, r)| *r)
                .unwrap_or(zone);
            crate::webview_ops::create_service(&app, label.clone(), url, rect).await?;
            let mut s = state.write().await;
            if !s.created_webview_labels.contains(&label) {
                s.created_webview_labels.push(label);
            }
        }
    }

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

    Ok((*state.read().await.service_tree).clone())
}

#[tauri::command]
pub async fn close_pane(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    pane_id: String,
) -> Result<LayoutNode, String> {
    let target = PaneId(pane_id);

    let service_id_to_hide = {
        let s = state.read().await;
        layout::find_pane(&s.service_tree, &target).and_then(|p| {
            if let PaneKind::Service(id) = &p.kind {
                if !id.is_empty() {
                    Some(id.clone())
                } else {
                    None
                }
            } else {
                None
            }
        })
    };

    let (new_tree, new_focus) = {
        let s = state.read().await;
        layout::close_pane_in_tree(&s.service_tree, &target)?
    };

    {
        let mut s = state.write().await;
        s.service_tree = Arc::new(new_tree);
        s.focused_pane_id = new_focus;
    }

    if let Some(svc_id) = service_id_to_hide {
        if let Some(wv) = app.get_webview(&format!("service-{}", svc_id)) {
            let _ = wv.hide();
        }
    }

    {
        let mut s = state.write().await;
        s.sync_tree_to_active_space();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

    Ok((*state.read().await.service_tree).clone())
}

#[tauri::command]
pub async fn resize_split(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    path: Vec<usize>,
    sizes: Vec<SplitSize>,
) -> Result<LayoutNode, String> {
    let new_tree = {
        let s = state.read().await;
        layout::apply_split_sizes(&s.service_tree, &path, sizes)?
    };
    {
        let mut s = state.write().await;
        s.service_tree = Arc::new(new_tree);
        s.sync_tree_to_active_space();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }
    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    Ok((*state.read().await.service_tree).clone())
}

#[tauri::command]
pub async fn focus_pane(state: State<'_, RwLock<AppState>>, pane_id: String) -> Result<(), String> {
    state.write().await.focused_pane_id = Some(PaneId(pane_id));
    Ok(())
}

#[tauri::command]
pub async fn apply_layout_preset(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    preset: LayoutPreset,
) -> Result<PaneTreeState, String> {
    let current_services = {
        let s = state.read().await;
        layout::collect_service_ids(&s.service_tree)
    };

    let preset_tree = layout::build_tree_for_preset(preset);
    let filled_tree = layout::fill_preset_with_services(preset_tree, &current_services);
    let new_focus = layout::first_pane_id(&filled_tree);

    {
        let mut s = state.write().await;
        s.service_tree = Arc::new(filled_tree);
        s.focused_pane_id = new_focus;
        if let Some(svc) = layout::collect_service_ids(&s.service_tree)
            .into_iter()
            .next()
        {
            s.active_service_id = svc;
        }
        s.sync_tree_to_active_space();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

    let s = state.read().await;
    Ok(PaneTreeState {
        tree: (*s.service_tree).clone(),
        focused_pane_id: s.focused_pane_id.as_ref().map(|id| id.0.clone()),
    })
}

#[tauri::command]
pub async fn switch_service_in_pane(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    pane_id: String,
    service_id: String,
) -> Result<PaneTreeState, String> {
    let target = PaneId(pane_id.clone());
    let label = format!("service-{}", service_id);

    // Duplicate check: reject if service is already in a different pane
    {
        let s = state.read().await;
        let current_in_target = layout::find_pane(&s.service_tree, &target).and_then(|p| {
            if let PaneKind::Service(id) = &p.kind {
                if !id.is_empty() {
                    Some(id.clone())
                } else {
                    None
                }
            } else {
                None
            }
        });
        if current_in_target.as_deref() != Some(&service_id)
            && layout::collect_service_ids(&s.service_tree).contains(&service_id)
        {
            return Err(format!(
                "service '{}' is already open in another pane",
                service_id
            ));
        }
    }

    // Lazy webview creation
    if app.get_webview(&label).is_none() {
        let (url_opt, zone_opt) = {
            let s = state.read().await;
            let url = s
                .services
                .iter()
                .find(|svc| svc.id == service_id)
                .map(|svc| svc.url.clone());
            let zone = app
                .get_window("main")
                .and_then(|w| webview_manager::get_viewport(&w))
                .map(|vp| layout::compute_service_zone_rect(vp, &s));
            (url, zone)
        };
        if let (Some(url), Some(zone)) = (url_opt, zone_opt) {
            crate::webview_ops::create_service(&app, label.clone(), url, zone).await?;
            let mut s = state.write().await;
            if !s.created_webview_labels.contains(&label) {
                s.created_webview_labels.push(label);
            }
        }
    }

    // Update tree
    {
        let mut s = state.write().await;
        let new_tree =
            layout::assign_service_to_pane(&s.service_tree, &target, service_id.clone())?;
        s.service_tree = Arc::new(new_tree);
        s.focused_pane_id = Some(target);
        s.active_service_id = service_id.clone();
        s.sync_tree_to_active_space();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }
    store::save_value(&app, "activeServiceId", &service_id);

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

    let s = state.read().await;
    Ok(PaneTreeState {
        tree: (*s.service_tree).clone(),
        focused_pane_id: s.focused_pane_id.as_ref().map(|id| id.0.clone()),
    })
}

// ========================================
// Space commands
// ========================================

#[tauri::command]
pub async fn get_spaces(state: State<'_, RwLock<AppState>>) -> Result<Vec<Space>, String> {
    Ok(state.read().await.spaces.clone())
}

#[tauri::command]
pub async fn get_active_space_id(state: State<'_, RwLock<AppState>>) -> Result<String, String> {
    Ok(state.read().await.active_space_id.clone())
}

#[tauri::command]
pub async fn create_space(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    name: String,
) -> Result<Vec<Space>, String> {
    let new_id = format!("space-{}", chrono_now());
    let new_space = Space {
        id: new_id.clone(),
        name: if name.is_empty() {
            let n = state.read().await.spaces.len() + 1;
            format!("スペース {}", n)
        } else {
            name
        },
        tree: crate::layout::build_tree_for_preset(LayoutPreset::Single),
    };

    let spaces = {
        let mut s = state.write().await;
        s.spaces.push(new_space);
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
        s.spaces.clone()
    };
    let _ = app.emit("space-list-updated", ());
    Ok(spaces)
}

#[tauri::command]
pub async fn delete_space(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    space_id: String,
) -> Result<Vec<Space>, String> {
    let (spaces, new_active) = {
        let mut s = state.write().await;
        if s.spaces.len() <= 1 {
            return Err("cannot delete the last space".into());
        }
        let idx = s
            .spaces
            .iter()
            .position(|sp| sp.id == space_id)
            .ok_or("space not found")?;
        s.spaces.remove(idx);

        // If we deleted the active space, switch to the first remaining
        if s.active_space_id == space_id {
            let new_id = s.spaces[0].id.clone();
            s.active_space_id = new_id.clone();
            s.load_active_space_tree();
            (s.spaces.clone(), new_id)
        } else {
            (s.spaces.clone(), s.active_space_id.clone())
        }
    };

    store::save_spaces(&app, &spaces, &new_active);

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());
    let _ = app.emit("space-list-updated", ());
    Ok(spaces)
}

#[tauri::command]
pub async fn rename_space(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    space_id: String,
    name: String,
) -> Result<Vec<Space>, String> {
    let spaces = {
        let mut s = state.write().await;
        let space = s
            .spaces
            .iter_mut()
            .find(|sp| sp.id == space_id)
            .ok_or("space not found")?;
        space.name = name;
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
        s.spaces.clone()
    };
    let _ = app.emit("space-list-updated", ());
    Ok(spaces)
}

#[tauri::command]
pub async fn switch_space(
    app: AppHandle,
    state: State<'_, RwLock<AppState>>,
    space_id: String,
) -> Result<PaneTreeState, String> {
    {
        let mut s = state.write().await;
        if s.active_space_id == space_id {
            return Ok(PaneTreeState {
                tree: (*s.service_tree).clone(),
                focused_pane_id: s.focused_pane_id.as_ref().map(|id| id.0.clone()),
            });
        }
        if !s.spaces.iter().any(|sp| sp.id == space_id) {
            return Err("space not found".into());
        }
        // Save current tree to active space before switching
        s.sync_tree_to_active_space();
        // Switch
        s.active_space_id = space_id.clone();
        s.load_active_space_tree();
        store::save_spaces(&app, &s.spaces, &s.active_space_id);
    }

    let snapshot = state.read().await.clone();
    webview_manager::update_layout(&app, &snapshot);
    let _ = app.emit("pane-tree-updated", ());

    let s = state.read().await;
    Ok(PaneTreeState {
        tree: (*s.service_tree).clone(),
        focused_pane_id: s.focused_pane_id.as_ref().map(|id| id.0.clone()),
    })
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
