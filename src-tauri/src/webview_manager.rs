use crate::layout::Rect;
use crate::state::AppState;
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

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

    // === window.open 等を同じウィンドウ内ナビゲーションに変換し、Rust 側の on_navigation で制御する ===
    var _origOpen = window.open;
    window.open = function(url, target, features) {
      if (url && url !== '' && url !== 'about:blank') {
        try {
          var fullUrl = new URL(url, window.location.href).href;
          console.log('[window.open -> same-window navigate]', fullUrl);
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
      // target="_blank" 等のリンクを同じウィンドウで開く（Rust の on_navigation でブラウザ送りにするため）
      document.addEventListener('click', function(e) {
        var link = e.target.closest && e.target.closest('a[target="_blank"], a[target="new"]');
        if (link && link.href) {
          e.preventDefault();
          e.stopPropagation();
          console.log('[target=_blank -> same-window navigate]', link.href);
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

/// Get viewport Rect from main window (logical pixels).
pub fn get_viewport(main_window: &tauri::Window) -> Option<crate::layout::Rect> {
    let window_size = main_window.inner_size().ok()?;
    let scale = main_window.scale_factor().unwrap_or(1.0);
    Some(crate::layout::Rect {
        x: 0.0,
        y: 0.0,
        width: (window_size.width as f64 / scale) as f32,
        height: (window_size.height as f64 / scale) as f32,
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

/// 同じサービスのドメイン内での移動（ログイン時のリダイレクト等）かどうかを判定する。
pub fn is_internal_navigation(nav_url: &url::Url, initial_url: &url::Url) -> bool {
    let nav_host = match nav_url.host_str() {
        Some(h) => h,
        None => return true, // スキームがない場合などは許可
    };
    let initial_host = match initial_url.host_str() {
        Some(h) => h,
        None => return true,
    };

    if nav_host == initial_host {
        return true;
    }

    // サービスごとに許可するドメイングループ
    let internal_domain_suffixes = [
        "slack.com",
        "google.com",
        "google.co.jp",
        "discord.com",
        "microsoft.com",
        "microsoftonline.com",
        "live.com",
        "chatwork.com",
    ];

    for suffix in internal_domain_suffixes {
        let is_nav_match = nav_host == suffix || nav_host.ends_with(&format!(".{}", suffix));
        let is_initial_match =
            initial_host == suffix || initial_host.ends_with(&format!(".{}", suffix));

        if is_nav_match && is_initial_match {
            return true;
        }
    }

    false
}

/// Create the chrome (dock) child Webview.
pub fn create_chrome_webview(
    main_window: &tauri::Window,
    rect: Rect,
) -> Result<tauri::Webview, String> {
    let webview_builder =
        tauri::WebviewBuilder::new("chrome", tauri::WebviewUrl::App("chrome.html".into()));
    main_window
        .add_child(
            webview_builder,
            tauri::LogicalPosition::new(rect.x as f64, rect.y as f64),
            tauri::LogicalSize::new(rect.width as f64, rect.height as f64),
        )
        .map_err(|e: tauri::Error| e.to_string())
}

/// Create a service Webview dynamically.
pub fn create_service_webview(
    app: &tauri::AppHandle,
    main_window: &tauri::Window,
    label: &str,
    url: &str,
    rect: Rect,
) -> Result<tauri::Webview, String> {
    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let service_id = label.strip_prefix("service-").unwrap_or(label);

    // Per-service data directory: persists cookies/session across restarts
    let safe_id: String = service_id
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let data_dir = app
        .path()
        .app_data_dir()
        .map(|p| p.join("webview-data").join(safe_id));

    let mut builder =
        tauri::WebviewBuilder::new(label, tauri::WebviewUrl::External(parsed_url.clone()))
            .user_agent(CHROME_USER_AGENT)
            .initialization_script(browser_spoof_script())
            .initialization_script(crate::notification::get_notification_script(service_id));

    if let Ok(dir) = data_dir {
        builder = builder.data_directory(dir);
    }

    let webview_builder = builder.on_navigation({
        let app_handle = app.clone();
        let initial_url = parsed_url.clone();
        move |nav_url| {
            println!("[service-nav] {}", nav_url.as_str());
            let is_internal = is_internal_navigation(nav_url, &initial_url);
            let is_auth = is_auth_url(nav_url.as_str());

            if !is_internal && !is_auth {
                println!(
                    "[service-nav] Opening external link in browser: {}",
                    nav_url.as_str()
                );
                #[allow(deprecated)]
                let _ = app_handle.shell().open(nav_url.as_str(), None);
                return false;
            }
            true
        }
    });

    let webview = main_window
        .add_child(
            webview_builder,
            tauri::LogicalPosition::new(rect.x as f64, rect.y as f64),
            tauri::LogicalSize::new(rect.width as f64, rect.height as f64),
        )
        .map_err(|e: tauri::Error| e.to_string())?;

    Ok(webview)
}

/// Create the AI companion Webview dynamically.
pub fn create_ai_webview(
    app: &tauri::AppHandle,
    main_window: &tauri::Window,
    url: &str,
    rect: Rect,
) -> Result<tauri::Webview, String> {
    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let webview_builder = tauri::WebviewBuilder::new(
        "ai-webview",
        tauri::WebviewUrl::External(parsed_url.clone()),
    )
    .user_agent(CHROME_USER_AGENT)
    .initialization_script(browser_spoof_script())
    .on_navigation({
        let app_handle = app.clone();
        let initial_url = parsed_url.clone();
        move |nav_url| {
            println!("[ai-nav] {}", nav_url.as_str());
            let is_internal = is_internal_navigation(nav_url, &initial_url);
            let is_auth = is_auth_url(nav_url.as_str());

            if !is_internal && !is_auth {
                println!(
                    "[ai-nav] Opening external link in browser: {}",
                    nav_url.as_str()
                );
                #[allow(deprecated)]
                let _ = app_handle.shell().open(nav_url.as_str(), None);
                return false;
            }
            true
        }
    });

    let webview = main_window
        .add_child(
            webview_builder,
            tauri::LogicalPosition::new(rect.x as f64, rect.y as f64),
            tauri::LogicalSize::new(rect.width as f64, rect.height as f64),
        )
        .map_err(|e: tauri::Error| e.to_string())?;

    Ok(webview)
}

/// Create the titlebar child Webview.
#[allow(dead_code)]
pub fn create_titlebar_webview(
    main_window: &tauri::Window,
    rect: Rect,
) -> Result<tauri::Webview, String> {
    let builder =
        tauri::WebviewBuilder::new("titlebar", tauri::WebviewUrl::App("titlebar.html".into()));
    main_window
        .add_child(
            builder,
            tauri::LogicalPosition::new(rect.x as f64, rect.y as f64),
            tauri::LogicalSize::new(rect.width as f64, rect.height as f64),
        )
        .map_err(|e| e.to_string())
}

/// Update layout of all child webviews (chrome + service tree + AI).
pub fn update_layout(app_handle: &tauri::AppHandle, state: &AppState) {
    let viewport = match app_handle.get_window("main").and_then(|w| get_viewport(&w)) {
        Some(v) => v,
        None => return,
    };

    let tb = crate::layout::TITLE_BAR_HEIGHT;
    let header_h = crate::layout::PANE_HEADER_HEIGHT;

    // Chrome (dock) — fixed rect below titlebar
    if let Some(wv) = app_handle.get_webview("chrome") {
        let _ = wv.set_position(tauri::LogicalPosition::new(0.0, tb as f64));
        let _ = wv.set_size(tauri::LogicalSize::new(
            crate::layout::DOCK_WIDTH as f64,
            (viewport.height - tb) as f64,
        ));
    }

    // Service tree — compute rects, position/show each leaf's webview
    let zone = crate::layout::compute_service_zone_rect(viewport, state);
    let pane_rects = crate::layout::compute_rects(&state.service_tree, zone);

    let present_ids: std::collections::HashSet<String> =
        crate::layout::collect_service_ids(&state.service_tree)
            .into_iter()
            .collect();

    for (pane_id, rect) in &pane_rects {
        if let Some(pane) = crate::layout::find_pane(&state.service_tree, pane_id) {
            if let crate::state::PaneKind::Service(svc_id) = &pane.kind {
                if svc_id.is_empty() {
                    continue;
                }
                let label = format!("service-{}", svc_id);
                if let Some(wv) = app_handle.get_webview(&label) {
                    let y = rect.y + header_h;
                    let h = (rect.height - header_h).max(0.0);
                    let _ = wv.set_position(tauri::LogicalPosition::new(rect.x as f64, y as f64));
                    let _ = wv.set_size(tauri::LogicalSize::new(rect.width as f64, h as f64));
                    let _ = wv.show();
                }
            }
        }
    }

    // Hide service webviews no longer in tree
    for label in &state.created_webview_labels {
        if let Some(svc_id) = label.strip_prefix("service-") {
            if !present_ids.contains(svc_id) {
                if let Some(wv) = app_handle.get_webview(label) {
                    let _ = wv.hide();
                }
            }
        }
    }

    // AI webview — fixed right panel, offset by RESIZE_GAP for the resize handle
    if state.ai_webview_created {
        if let Some(wv) = app_handle.get_webview("ai-webview") {
            if state.show_ai_companion {
                let gap = crate::layout::RESIZE_GAP;
                let ai_x = zone.x + zone.width + gap;
                let _ = wv.set_position(tauri::LogicalPosition::new(ai_x as f64, tb as f64));
                let _ = wv.set_size(tauri::LogicalSize::new(
                    (state.ai_width as f32).max(0.0) as f64,
                    (viewport.height - tb) as f64,
                ));
                let _ = wv.show();
            } else {
                let _ = wv.hide();
            }
        }
    }
}

/// Called when the main window is resized.
#[allow(dead_code)]
pub fn on_main_window_resized(app_handle: &tauri::AppHandle) {
    let state = app_handle.state::<tokio::sync::RwLock<AppState>>();
    let s = state.blocking_read();
    update_layout(app_handle, &s);
}
