mod commands;
mod layout;
mod notification;
mod state;
mod store;
mod tray;
mod webview_manager;
mod webview_ops;

use state::AppState;
use tauri::Manager;
use tokio::sync::RwLock;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_services,
            commands::add_service,
            commands::remove_service,
            commands::update_service,
            commands::reorder_services,
            commands::get_active_service,
            commands::set_active_service,
            commands::get_ai_services,
            commands::get_active_ai_service,
            commands::set_active_ai_service,
            commands::add_ai_service,
            commands::remove_ai_service,
            commands::get_show_ai_companion,
            commands::set_show_ai_companion,
            commands::get_ai_width,
            commands::set_ai_width,
            commands::get_platform,
            commands::window_start_drag,
            commands::window_minimize,
            commands::window_maximize,
            commands::window_close,
            commands::window_is_maximized,
            commands::create_service_webview,
            commands::switch_service_webview,
            commands::remove_service_webview,
            commands::create_ai_webview,
            commands::toggle_ai_webview,
            commands::resize_ai_webview,
            commands::update_layout,
            commands::create_all_service_webviews,
            commands::setup_ai_webview,
            commands::switch_ai_service,
            commands::send_to_ai_webview,
            commands::update_notification_count,
            commands::update_favicon,
            commands::open_popup_window,
            commands::hide_all_child_webviews,
            commands::restore_child_webviews,
            commands::request_open_modal,
        ])
        .on_window_event(|window, event| {
            let app_handle = window.app_handle();

            if let tauri::WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    if let Some(ww) = app_handle.get_webview_window("main") {
                        if let Ok(size) = ww.inner_size() {
                            let state = app_handle.state::<RwLock<AppState>>();
                            let mut s = state.blocking_write();
                            s.window_bounds.width = size.width;
                            s.window_bounds.height = size.height;
                            store::save_state(app_handle, &s);
                        }
                    }
                }
            }
        })
        .setup(|app| {
            // Load persisted state and register it
            let app_state = store::load_state(app.handle());
            app.manage(RwLock::new(app_state));

            // Setup system tray
            tray::setup_tray(app)?;

            // Create all service webview windows in setup (main thread)
            // This avoids the deadlock that occurs when creating windows from command threads
            {
                // Collect service info first, then release the lock
                let (services_info, active_service_id) = {
                    let state = app.state::<RwLock<AppState>>();
                    let s = state.blocking_read();
                    let services: Vec<(String, String)> = s
                        .services
                        .iter()
                        .filter(|svc| svc.enabled)
                        .map(|svc| (svc.id.clone(), svc.url.clone()))
                        .collect();
                    let active = s.active_service_id.clone();
                    (services, active)
                };
                // Lock released

                let main_win = app.get_window("main");

                if let Some(ref main_win) = main_win {
                    let viewport = webview_manager::get_viewport(main_win);

                    if let Some(vp) = viewport {
                        let (chrome_rect, svc_rect_opt) = {
                            let state = app.state::<RwLock<AppState>>();
                            let s = state.blocking_read();
                            let tree = layout::build_tree_from_state(&s);
                            let rects = layout::compute_rects(&tree, vp);

                            let chrome_r = rects
                                .iter()
                                .find(|(id, _)| id.0 == "chrome")
                                .map(|(_, r)| *r);

                            let active_key = format!("service:{}", active_service_id);
                            let svc_r = rects
                                .iter()
                                .find(|(id, _)| id.0 == active_key)
                                .map(|(_, r)| *r);

                            (chrome_r, svc_r)
                        };

                        let mut created_labels: Vec<String> = Vec::new();

                        // Create chrome (dock) webview first — always visible
                        if let Some(r) = chrome_rect {
                            println!("[setup] Creating chrome webview");
                            match webview_ops::create_chrome_inline(app.handle(), r) {
                                Ok(_) => println!("[setup] Created chrome webview OK"),
                                Err(e) => {
                                    println!("[setup] Failed to create chrome webview: {}", e)
                                }
                            }
                        }

                        // Create ONLY the active service webview.
                        // Other services are created lazily on first switch.
                        if let Some((service_id, url)) = services_info
                            .iter()
                            .find(|(id, _)| *id == active_service_id)
                            .or_else(|| services_info.first())
                        {
                            let label = format!("service-{}", service_id);
                            let tb = layout::TITLE_BAR_HEIGHT;
                            let r = svc_rect_opt.unwrap_or(crate::layout::Rect {
                                x: layout::DOCK_WIDTH,
                                y: tb,
                                width: vp.width - layout::DOCK_WIDTH,
                                height: vp.height - tb,
                            });
                            println!("[setup] Creating active service webview: {}", label);
                            match webview_ops::create_service_inline(app.handle(), &label, url, r) {
                                Ok(wv) => {
                                    let _ = wv.show();
                                    created_labels.push(label);
                                    println!("[setup] Created active service webview OK");
                                }
                                Err(e) => println!(
                                    "[setup] Failed to create active service webview: {}",
                                    e
                                ),
                            }
                        }

                        // AI webview is created lazily via setup_ai_webview command.

                        // Update state with created labels
                        {
                            let state = app.state::<RwLock<AppState>>();
                            let mut s = state.blocking_write();
                            for label in created_labels {
                                if !s.created_webview_labels.contains(&label) {
                                    s.created_webview_labels.push(label);
                                }
                            }
                        }
                    }
                }
            }

            // Open DevTools in dev mode
            #[cfg(debug_assertions)]
            if let Some(ww) = app.get_webview_window("main") {
                ww.open_devtools();
            }

            // Start notification polling for external webviews
            // let app_handle = app.handle().clone();
            // std::thread::spawn(move || {
            //     notification::start_title_polling(app_handle);
            // });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
