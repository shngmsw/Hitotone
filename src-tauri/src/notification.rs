/// Generate the notification detection JavaScript that will be injected into service webviews.
/// This script monitors document.title changes and sends notification count updates.
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
        let num_start = before
            .rfind(|c: char| !c.is_ascii_digit())
            .map_or(0, |p| p + 1);
        let num_str = &before[num_start..];
        return num_str.parse().ok();
    }
    None
}
