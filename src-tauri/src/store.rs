use crate::state::{AiService, AppState, Service, WindowBounds};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const STORE_FILE: &str = "hitotone-config.json";

pub fn load_state(app: &AppHandle, state: &Mutex<AppState>) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut s = state.lock().unwrap();

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
}

pub fn save_state(app: &AppHandle, state: &AppState) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };

    let _ = store.set(
        "services",
        serde_json::to_value(&state.services).unwrap_or_default(),
    );
    let _ = store.set(
        "aiServices",
        serde_json::to_value(&state.ai_services).unwrap_or_default(),
    );
    let _ = store.set(
        "activeServiceId",
        serde_json::Value::String(state.active_service_id.clone()),
    );
    let _ = store.set(
        "activeAiServiceId",
        serde_json::Value::String(state.active_ai_service_id.clone()),
    );
    let _ = store.set(
        "showAiCompanion",
        serde_json::Value::Bool(state.show_ai_companion),
    );
    let _ = store.set(
        "aiWidth",
        serde_json::Value::Number(serde_json::Number::from(state.ai_width)),
    );
    let _ = store.set(
        "windowBounds",
        serde_json::to_value(&state.window_bounds).unwrap_or_default(),
    );

    let _ = store.save();
}

pub fn save_services(app: &AppHandle, services: &[Service]) {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = store.set(
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
    let _ = store.set(
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
    let _ = store.set(key, serde_json::to_value(value).unwrap_or_default());
    let _ = store.save();
}
