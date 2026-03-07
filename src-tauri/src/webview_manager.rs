use crate::state::AppState;
use tauri::Manager;

const DOCK_WIDTH: f64 = 64.0;
const HEADER_HEIGHT: f64 = 32.0;

/// Chrome-compatible User-Agent strings (platform-specific).
/// Slack や Google Chat は UA ヘッダだけでなく JS の navigator API でもブラウザを判定する。
/// プラットフォームに合った Chrome UA を設定し、初期化スクリプトで navigator.userAgentData も偽装する。
#[cfg(target_os = "macos")]
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
#[cfg(target_os = "windows")]
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const CHROME_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

/// Chrome を完全に偽装するための JS 初期化スクリプト。
/// navigator.userAgentData (Client Hints API) がないと Slack 等に弾かれるため追加する。
const BROWSER_SPOOF_SCRIPT: &str = r#"
(function() {
  // === navigator 偽装 (DOM 不要、最優先で実行) ===
  Object.defineProperty(navigator, 'userAgent', {
    get: function() { return 'USER_AGENT_PLACEHOLDER'; },
    configurable: true
  });
  Object.defineProperty(navigator, 'userAgentData', {
    get: function() {
      return {
        brands: [
          { brand: 'Chromium', version: '134' },
          { brand: 'Google Chrome', version: '134' },
          { brand: 'Not:A-Brand', version: '99' }
        ],
        mobile: false,
        platform: 'PLATFORM_PLACEHOLDER',
        getHighEntropyValues: function(hints) {
          return Promise.resolve({
            brands: this.brands, mobile: false,
            platform: 'PLATFORM_PLACEHOLDER', platformVersion: '15.0.0',
            architecture: 'x86', bitness: '64',
            fullVersionList: [
              { brand: 'Chromium', version: '134.0.6998.89' },
              { brand: 'Google Chrome', version: '134.0.6998.89' },
              { brand: 'Not:A-Brand', version: '99.0.0.0' }
            ],
            model: '', uaFullVersion: '134.0.6998.89'
          });
        }
      };
    },
    configurable: true
  });
  Object.defineProperty(navigator, 'vendor', {
    get: function() { return 'Google Inc.'; },
    configurable: true
  });
  Object.defineProperty(navigator, 'appVersion', {
    get: function() { return '5.0 (PLATFORM_UA_PLACEHOLDER)'; },
    configurable: true
  });

  // === window.open を同じウィンドウ内ナビゲーションに変換 (DOM 不要) ===
  var _origOpen = window.open;
  window.open = function(url, target, features) {
    if (url && url !== '' && url !== 'about:blank') {
      try {
        var fullUrl = new URL(url, window.location.href).href;
        console.log('[window.open -> navigate]', fullUrl);
        window.location.href = fullUrl;
      } catch(e) {
        console.error('[window.open] error:', e);
      }
      return null;
    }
    return _origOpen ? _origOpen.call(window, url, target, features) : null;
  };

  // === DOM 準備後に実行する処理 ===
  function onReady() {
    // target="_blank" リンクを同じウィンドウで開く
    document.addEventListener('click', function(e) {
      var link = e.target.closest && e.target.closest('a[target="_blank"], a[target="new"]');
      if (link && link.href) {
        e.preventDefault();
        e.stopPropagation();
        console.log('[target=_blank -> navigate]', link.href);
        window.location.href = link.href;
      }
    }, true);

    // 非推奨バナーを非表示にする CSS
    var style = document.createElement('style');
    style.textContent = [
      '[data-qa="browser_deprecation_banner"] { display: none !important; }',
      '.p-browser_deprecation_banner { display: none !important; }',
      '.c-banner--warning { display: none !important; }',
      '[data-unsupported-browser] { display: none !important; }',
      '.unsupported-browser-banner { display: none !important; }'
    ].join('\n');
    (document.head || document.documentElement).appendChild(style);

    // SPA 対応: DOM 更新時にもバナーを消す
    if (document.body) {
      var observer = new MutationObserver(function() {
        document.querySelectorAll(
          '[data-qa="browser_deprecation_banner"], .p-browser_deprecation_banner, .c-banner--warning'
        ).forEach(function(el) { el.remove(); });
      });
      observer.observe(document.body, { childList: true, subtree: true });
    }
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', onReady);
  } else {
    onReady();
  }
})();
"#;

/// プラットフォームに合わせて JS スクリプト内のプレースホルダを置換
pub fn browser_spoof_script() -> String {
    #[cfg(target_os = "macos")]
    let (platform, platform_ua) = ("macOS", "Macintosh; Intel Mac OS X 10_15_7");
    #[cfg(target_os = "windows")]
    let (platform, platform_ua) = ("Windows", "Windows NT 10.0; Win64; x64");
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let (platform, platform_ua) = ("Linux", "X11; Linux x86_64");

    BROWSER_SPOOF_SCRIPT
        .replace("USER_AGENT_PLACEHOLDER", CHROME_USER_AGENT)
        .replace("PLATFORM_PLACEHOLDER", platform)
        .replace("PLATFORM_UA_PLACEHOLDER", platform_ua)
}

/// Chrome UA 文字列を返す
pub fn chrome_user_agent() -> &'static str {
    CHROME_USER_AGENT
}

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

/// 外部 OAuth プロバイダの URL かどうかを判定する。
/// Slack 自身の /signin, /sso, /oauth 等はポップアップにせず WebView 内で処理させる。
/// ポップアップを開くのは Google, Microsoft 等の外部認証画面のみ。
pub fn is_auth_url(url: &str) -> bool {
    url.contains("accounts.google.com")
        || url.contains("login.microsoftonline.com")
        || url.contains("github.com/login/oauth")
        || url.contains("appleid.apple.com/auth")
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
    .user_agent(CHROME_USER_AGENT)
    .initialization_script(&browser_spoof_script())
    .on_navigation(move |nav_url| {
        println!("[service-nav] {}", nav_url.as_str());
        true // すべてのナビゲーションをそのまま許可（ポップアップ分離はしない）
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
    .user_agent(CHROME_USER_AGENT)
    .initialization_script(&browser_spoof_script())
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
