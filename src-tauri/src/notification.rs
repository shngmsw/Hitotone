use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
use tauri::{Emitter, Manager};

/// Generate the notification detection JavaScript that will be injected into service webviews.
/// This script monitors document.title changes and sends notification count updates.
#[allow(dead_code)]
pub fn get_notification_script(service_id: &str) -> String {
    format!(
        r#"(function() {{
  const SERVICE_ID = '{}';
  let lastCount = 0;
  const patterns = [/\((\d+)\)/, /\[(\d+)\]/, /(\d+)\s*new/, /(\d+)\s*unread/];

  function extractCount(title) {{
    for (const p of patterns) {{
      const m = title.match(p);
      if (m) return Math.min(parseInt(m[1], 10), 9999);
    }}
    return 0;
  }}

  function sendCount(count) {{
    if (window.__TAURI_INTERNALS__) {{
      window.__TAURI_INTERNALS__.invoke('update_notification_count', {{
        serviceId: SERVICE_ID,
        count: count
      }});
    }}
  }}

  // Use MutationObserver to watch title changes
  const observer = new MutationObserver(function() {{
    const count = extractCount(document.title || '');
    if (count !== lastCount) {{
      lastCount = count;
      sendCount(count);
    }}
  }});

  const titleEl = document.querySelector('title');
  if (titleEl) {{
    observer.observe(titleEl, {{ childList: true, characterData: true, subtree: true }});
  }}

  // Initial check
  const initialCount = extractCount(document.title || '');
  if (initialCount > 0) {{
    lastCount = initialCount;
    sendCount(initialCount);
  }}
}})();"#,
        service_id
    )
}

/// Fallback: poll document.title from Rust side for external webviews
/// where Tauri IPC may not be available.
pub fn start_title_polling(app_handle: tauri::AppHandle) {
    let _title_cache: StdMutex<HashMap<String, u32>> = StdMutex::new(HashMap::new());
    let _favicon_sent: StdMutex<HashMap<String, String>> = StdMutex::new(HashMap::new());

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Get list of service webview labels
        let labels: Vec<String> = {
            let state = app_handle.state::<std::sync::Mutex<crate::state::AppState>>();
            let s = state.lock().unwrap();
            s.created_webview_labels.clone()
        };

        for label in &labels {
            if let Some(ww) = app_handle.get_webview_window(label) {
                let service_id = label.strip_prefix("service-").unwrap_or(label);

                // Notification count polling via eval
                let _ = ww.eval(&format!(
                    r#"(function() {{
                        const patterns = [/\((\d+)\)/, /\[(\d+)\]/, /(\d+)\s*new/, /(\d+)\s*unread/];
                        const title = document.title || '';
                        let count = 0;
                        for (const p of patterns) {{
                            const m = title.match(p);
                            if (m) {{ count = Math.min(parseInt(m[1], 10), 9999); break; }}
                        }}
                        if (window.__TAURI_INTERNALS__) {{
                            window.__TAURI_INTERNALS__.invoke('update_notification_count', {{
                                serviceId: '{}',
                                count: count
                            }});
                        }}
                    }})();"#,
                    service_id
                ));

                // Favicon: derive from service URL using Google's favicon service
                {
                    let state = app_handle.state::<std::sync::Mutex<crate::state::AppState>>();
                    let s = state.lock().unwrap();
                    if let Some(svc) = s.services.iter().find(|svc| format!("service-{}", svc.id) == *label) {
                        if svc.favicon_url.is_none() {
                            if let Ok(parsed) = svc.url.parse::<url::Url>() {
                                if let Some(domain) = parsed.host_str() {
                                    let favicon_url = format!(
                                        "https://www.google.com/s2/favicons?domain={}&sz=32",
                                        domain
                                    );
                                    drop(s);
                                    let _ = app_handle.emit("favicon-updated", serde_json::json!({
                                        "serviceId": service_id,
                                        "faviconUrl": favicon_url
                                    }));
                                    // Save it
                                    let state2 = app_handle.state::<std::sync::Mutex<crate::state::AppState>>();
                                    let mut s2 = state2.lock().unwrap();
                                    if let Some(svc2) = s2.services.iter_mut().find(|sv| format!("service-{}", sv.id) == *label) {
                                        svc2.favicon_url = Some(favicon_url);
                                    }
                                    crate::store::save_services(&app_handle, &s2.services);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Parse notification count from a title string
#[allow(dead_code)]
pub fn parse_title_count(title: &str) -> u32 {
    if let Some(caps) = regex_lite_match(title, r"\((\d+)\)") {
        return caps.min(9999);
    }
    if let Some(caps) = regex_lite_match(title, r"\[(\d+)\]") {
        return caps.min(9999);
    }
    if let Some(caps) = regex_lite_match(title, r"(\d+)\s*new") {
        return caps.min(9999);
    }
    if let Some(caps) = regex_lite_match(title, r"(\d+)\s*unread") {
        return caps.min(9999);
    }
    0
}

#[allow(dead_code)]
fn regex_lite_match(text: &str, pattern: &str) -> Option<u32> {
    match pattern {
        r"\((\d+)\)" => {
            if let Some(start) = text.find('(') {
                if let Some(end) = text[start..].find(')') {
                    let num_str = &text[start + 1..start + end];
                    return num_str.parse().ok();
                }
            }
            None
        }
        r"\[(\d+)\]" => {
            if let Some(start) = text.find('[') {
                if let Some(end) = text[start..].find(']') {
                    let num_str = &text[start + 1..start + end];
                    return num_str.parse().ok();
                }
            }
            None
        }
        r"(\d+)\s*new" => extract_number_before(text, "new"),
        r"(\d+)\s*unread" => extract_number_before(text, "unread"),
        _ => None,
    }
}

#[allow(dead_code)]
fn extract_number_before(text: &str, keyword: &str) -> Option<u32> {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(keyword) {
        let before = text[..pos].trim_end();
        let num_start = before.rfind(|c: char| !c.is_ascii_digit()).map_or(0, |p| p + 1);
        let num_str = &before[num_start..];
        return num_str.parse().ok();
    }
    None
}
