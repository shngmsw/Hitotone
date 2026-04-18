use crate::state::{AiService, AppState, Service, Space, WindowBounds};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "hitotone-config.json";

pub fn load_state(app: &AppHandle) -> AppState {
    let mut s = AppState::default();

    let store = match app.store(STORE_FILE) {
        Ok(store) => store,
        Err(_) => return s,
    };

    if let Some(val) = store.get("services") {
        if let Ok(services) = serde_json::from_value::<Vec<Service>>(val.clone()) {
            s.services = services;
        }
    }

    if let Some(val) = store.get("aiServices") {
        if let Ok(ai_services) = serde_json::from_value::<Vec<AiService>>(val.clone()) {
            s.ai_services = ai_services;
        }
    }

    if let Some(val) = store.get("activeServiceId") {
        if let Some(id) = val.as_str() {
            s.active_service_id = id.to_string();
        }
    }

    if let Some(val) = store.get("activeAiServiceId") {
        if let Some(id) = val.as_str() {
            s.active_ai_service_id = id.to_string();
        }
    }

    if let Some(val) = store.get("showAiCompanion") {
        if let Some(show) = val.as_bool() {
            s.show_ai_companion = show;
        }
    }

    if let Some(val) = store.get("aiWidth") {
        if let Some(width) = val.as_u64() {
            s.ai_width = width as u32;
        }
    }

    if let Some(val) = store.get("windowBounds") {
        if let Ok(bounds) = serde_json::from_value::<WindowBounds>(val.clone()) {
            s.window_bounds = bounds;
        }
    }

    if let Some(val) = store.get("spaces") {
        if let Ok(spaces) = serde_json::from_value::<Vec<Space>>(val.clone()) {
            if !spaces.is_empty() {
                s.spaces = spaces;
            }
        }
    }

    if let Some(val) = store.get("activeSpaceId") {
        if let Some(id) = val.as_str() {
            s.active_space_id = id.to_string();
        }
    }

    // Ensure active_space_id points to a valid space; fall back to first
    if !s.spaces.iter().any(|sp| sp.id == s.active_space_id) {
        if let Some(first) = s.spaces.first() {
            s.active_space_id = first.id.clone();
        }
    }

    // Initialize service_tree from active space tree
    s.load_active_space_tree();

    s
}

pub fn save_state(app: &AppHandle, state: &AppState) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };

    store.set(
        "services",
        serde_json::to_value(&state.services).unwrap_or_default(),
    );
    store.set(
        "aiServices",
        serde_json::to_value(&state.ai_services).unwrap_or_default(),
    );
    store.set(
        "activeServiceId",
        serde_json::Value::String(state.active_service_id.clone()),
    );
    store.set(
        "activeAiServiceId",
        serde_json::Value::String(state.active_ai_service_id.clone()),
    );
    store.set(
        "showAiCompanion",
        serde_json::Value::Bool(state.show_ai_companion),
    );
    store.set(
        "aiWidth",
        serde_json::Value::Number(serde_json::Number::from(state.ai_width)),
    );
    store.set(
        "windowBounds",
        serde_json::to_value(&state.window_bounds).unwrap_or_default(),
    );
    store.set(
        "spaces",
        serde_json::to_value(&state.spaces).unwrap_or_default(),
    );
    store.set(
        "activeSpaceId",
        serde_json::Value::String(state.active_space_id.clone()),
    );

    let _ = store.save();
}

pub fn save_spaces(app: &AppHandle, spaces: &[Space], active_space_id: &str) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };
    store.set("spaces", serde_json::to_value(spaces).unwrap_or_default());
    store.set(
        "activeSpaceId",
        serde_json::Value::String(active_space_id.to_string()),
    );
    let _ = store.save();
}

pub fn save_services(app: &AppHandle, services: &[Service]) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };
    store.set(
        "services",
        serde_json::to_value(services).unwrap_or_default(),
    );
    let _ = store.save();
}

pub fn save_ai_services(app: &AppHandle, ai_services: &[AiService]) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };
    store.set(
        "aiServices",
        serde_json::to_value(ai_services).unwrap_or_default(),
    );
    let _ = store.save();
}

pub fn save_value<T: serde::Serialize>(app: &AppHandle, key: &str, value: &T) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };
    store.set(key, serde_json::to_value(value).unwrap_or_default());
    let _ = store.save();
}
