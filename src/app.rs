use crate::components::config_form::ConfigForm;
use crate::components::log_viewer::LogViewer;
use crate::components::toast::{Toast, ToastMessage, ToastType};
use crate::types::{CommandResponse, LogType, RustFsConfig};
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use std::collections::VecDeque;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

// Import CSS styles
const LOGS_CSS: &str = include_str!("logs.css");

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Helper function to check if we're in Tauri environment
fn is_tauri() -> bool {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &"__TAURI__".into()).ok())
        .map(|v| !v.is_undefined())
        .unwrap_or(false)
}

fn load_config() -> RustFsConfig {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("rustfs_config") {
                if let Ok(config) = serde_json::from_str(&json) {
                    return config;
                }
            }
        }
    }
    RustFsConfig::default()
}

fn save_config(config: &RustFsConfig) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(json) = serde_json::to_string(config) {
                let _ = storage.set_item("rustfs_config", &json);
            }
        }
    }
}

const APP_LOG_CAPACITY: usize = 100;
const RUSTFS_LOG_CAPACITY: usize = 1000;

fn push_log(writer: WriteSignal<VecDeque<String>>, msg: String, capacity: usize) {
    writer.update(|logs| {
        logs.push_back(msg);
        if logs.len() > capacity {
            logs.pop_front();
        }
    });
}

#[component]
pub fn App() -> impl IntoView {
    let (config, set_config) = signal(load_config());

    Effect::new(move |_| {
        save_config(&config.get());
    });

    let (toasts, set_toasts) = signal(Vec::<ToastMessage>::new());
    let (is_running, set_is_running) = signal(false);
    let (app_logs, set_app_logs) = signal(VecDeque::<String>::new());
    let (rustfs_logs, set_rustfs_logs) = signal(VecDeque::<String>::new());
    let (current_log_type, set_current_log_type) = signal(LogType::App);
    let (service_status, set_service_status) = signal(false);

    let remove_toast = Callback::new(move |id: u64| {
        set_toasts.update(|current| {
            current.retain(|t| t.id != id);
        });
    });

    let show_toast = move |message: String, toast_type: ToastType| {
        let id = js_sys::Date::now() as u64;
        set_toasts.update(|current| {
            current.push(ToastMessage {
                message,
                toast_type,
                id,
            });
        });

        // Auto remove after 3 seconds
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                Closure::once_into_js(move || {
                    remove_toast.run(id);
                })
                .unchecked_ref(),
                3000,
            );
    };

    // Health Check Polling
    Effect::new(move |_| {
        spawn_local(async move {
            if !is_tauri() {
                return;
            }

            let closure = Closure::wrap(Box::new(move || {
                spawn_local(async move {
                    let current = config.get_untracked();
                    if let Some(port) = current.port {
                        let host = current.host.unwrap_or_else(|| "127.0.0.1".to_string());

                        let args = js_sys::Object::new();
                        js_sys::Reflect::set(&args, &"host".into(), &host.into()).unwrap();
                        js_sys::Reflect::set(&args, &"port".into(), &port.into()).unwrap();

                        match tauri_invoke("check_tcp_connection", args.into())
                            .await
                            .as_bool()
                        {
                            Some(is_active) => set_service_status.set(is_active),
                            None => set_service_status.set(false),
                        }
                    }
                });
            }) as Box<dyn FnMut()>);

            let _ = web_sys::window()
                .unwrap()
                .set_interval_with_callback_and_timeout_and_arguments_0(
                    closure.as_ref().unchecked_ref(),
                    3000, // 3 seconds
                );

            closure.forget(); // Leak the closure to keep it alive
        });
    });

    let app_log_writer = set_app_logs;
    let rustfs_log_writer = set_rustfs_logs;

    spawn_local(async move {
        if !is_tauri() {
            push_log(
                app_log_writer,
                "[WARN] Not running in Tauri environment - logs disabled".to_string(),
                APP_LOG_CAPACITY,
            );
            return;
        }

        push_log(
            app_log_writer,
            "[DEBUG] Setting up real-time log listeners...".to_string(),
            APP_LOG_CAPACITY,
        );

        const APP_LOG_EVENT: &str = "app-log";
        const RUSTFS_LOG_EVENT: &str = "rustfs-log";
        const RUSTFS_EXIT_EVENT: &str = "rustfs-exit";

        fn create_log_listener(
            logs_signal: WriteSignal<VecDeque<String>>,
            max_logs: usize,
        ) -> Closure<dyn FnMut(JsValue)> {
            Closure::wrap(Box::new(move |event: JsValue| {
                if let Ok(payload) = js_sys::Reflect::get(&event, &"payload".into()) {
                    if let Some(log) = payload.as_string() {
                        push_log(logs_signal, log, max_logs);
                    }
                }
            }) as Box<dyn FnMut(JsValue)>)
        }

        let exit_listener = Closure::wrap(Box::new(move |event: JsValue| {
            if let Ok(payload) = js_sys::Reflect::get(&event, &"payload".into()) {
                if let Some(exit_code) = payload.as_string() {
                    set_is_running.set(false);
                    set_service_status.set(false);
                    show_toast(format!("RustFS exited with code: {}", exit_code), ToastType::Error);

                    // Log it
                    push_log(
                        app_log_writer,
                        format!("[ERROR] RustFS process exited unexpectedly: {}", exit_code),
                        APP_LOG_CAPACITY,
                    );

                    // Switch to RustFS logs so user sees why
                    set_current_log_type.set(LogType::RustFS);
                }
            }
        }) as Box<dyn FnMut(JsValue)>);

        if let Some(window) = web_sys::window() {
            let app_listener = create_log_listener(app_log_writer, APP_LOG_CAPACITY);
            let rustfs_listener = create_log_listener(rustfs_log_writer, RUSTFS_LOG_CAPACITY);

            if let Ok(tauri) = js_sys::Reflect::get(&window, &"__TAURI__".into()) {
                if let Ok(event) = js_sys::Reflect::get(&tauri, &"event".into()) {
                    if let Ok(listen) = js_sys::Reflect::get(&event, &"listen".into()) {
                        let listen_fn = js_sys::Function::from(listen);

                        let _ = listen_fn.call2(
                            &event,
                            &APP_LOG_EVENT.into(),
                            app_listener.as_ref().unchecked_ref(),
                        );
                        let _ = listen_fn.call2(
                            &event,
                            &RUSTFS_LOG_EVENT.into(),
                            rustfs_listener.as_ref().unchecked_ref(),
                        );
                        let _ = listen_fn.call2(
                            &event,
                            &RUSTFS_EXIT_EVENT.into(),
                            exit_listener.as_ref().unchecked_ref(),
                        );
                    }
                }
            }

            app_listener.forget();
            rustfs_listener.forget();
            exit_listener.forget();
        }

        // Fetch initial logs
        let app_logs_value = tauri_invoke("get_app_logs", js_sys::Object::new().into()).await;
        if let Ok(logs_vec) = serde_wasm_bindgen::from_value::<Vec<String>>(app_logs_value) {
            app_log_writer.set(logs_vec.into_iter().collect());
        }

        let rustfs_logs_value = tauri_invoke("get_rustfs_logs", js_sys::Object::new().into()).await;
        if let Ok(logs_vec) = serde_wasm_bindgen::from_value::<Vec<String>>(rustfs_logs_value) {
            rustfs_log_writer.set(logs_vec.into_iter().collect());
        }
    });

    let launch_rustfs = move |ev: SubmitEvent| {
        ev.prevent_default();
        set_is_running.set(true);
        show_toast("Launching RustFS...".to_string(), ToastType::Info);

        let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
        push_log(
            set_app_logs,
            format!("[{}] Launch button clicked", now),
            APP_LOG_CAPACITY,
        );
        push_log(
            set_app_logs,
            format!("[{}] Config: {:?}", now, config.get()),
            APP_LOG_CAPACITY,
        );

        spawn_local(async move {
            // Check if we're in Tauri environment
            if !is_tauri() {
                show_toast("Error: Not running in Tauri environment".to_string(), ToastType::Error);
                push_log(
                    set_app_logs,
                    "[ERROR] Not running in Tauri environment".to_string(),
                    APP_LOG_CAPACITY,
                );
                set_is_running.set(false);
                return;
            }

            let current_config = config.get_untracked();

            // 添加详细日志
            leptos::logging::log!(
                "Starting RustFS with config: data_path={}, port={:?}, host={:?}",
                current_config.data_path,
                current_config.port,
                current_config.host
            );

            let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
            push_log(
                set_app_logs,
                format!("[{}] Calling tauri_invoke with command: launch_rustfs", now),
                APP_LOG_CAPACITY,
            );

            // Create args object with config parameter
            let args = js_sys::Object::new();
            let config_js = serde_wasm_bindgen::to_value(&current_config).unwrap();
            js_sys::Reflect::set(&args, &"config".into(), &config_js).unwrap();

            let result_value = tauri_invoke("launch_rustfs", args.into()).await;
            let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
            push_log(
                set_app_logs,
                format!("[{}] Invoke result: {:?}", now, result_value),
                APP_LOG_CAPACITY,
            );

            match serde_wasm_bindgen::from_value::<CommandResponse>(result_value) {
                Ok(CommandResponse { success, message }) => {
                    let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
                    push_log(
                        set_app_logs,
                        format!("[{}] Result message: {}", now, message),
                        APP_LOG_CAPACITY,
                    );

                    if success {
                        show_toast("RustFS launched successfully!".to_string(), ToastType::Success);
                        let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
                        push_log(
                            set_app_logs,
                            format!("[{}] Launch successful!", now),
                            APP_LOG_CAPACITY,
                        );
                    } else {
                        show_toast(format!("Launch failed: {}", message), ToastType::Error);
                        let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
                        push_log(
                            set_app_logs,
                            format!("[{}] Launch result: {}", now, message),
                            APP_LOG_CAPACITY,
                        );
                        set_is_running.set(false);
                    }
                }
                Err(_) => {
                    show_toast("RustFS launch command failed".to_string(), ToastType::Error);
                    let now = js_sys::Date::new_0().to_locale_time_string("en-US".into());
                    push_log(
                        set_app_logs,
                        format!("[{}] Launch completed but response parsing failed", now),
                        APP_LOG_CAPACITY,
                    );
                    set_is_running.set(false);
                }
            }
        });
    };

    let stop_rustfs = move |_| {
        show_toast("Stopping RustFS...".to_string(), ToastType::Info);
        push_log(set_app_logs, "Stopping RustFS...".to_string(), APP_LOG_CAPACITY);

        spawn_local(async move {
            let result_value = tauri_invoke("stop_rustfs", js_sys::Object::new().into()).await;
            
            match serde_wasm_bindgen::from_value::<CommandResponse>(result_value) {
                Ok(res) => {
                    if res.success {
                        set_is_running.set(false);
                        set_service_status.set(false);
                        show_toast("RustFS stopped".to_string(), ToastType::Success);
                        push_log(set_app_logs, "RustFS stopped successfully".to_string(), APP_LOG_CAPACITY);
                    } else {
                        show_toast(format!("Failed to stop: {}", res.message), ToastType::Error);
                        push_log(set_app_logs, format!("Failed to stop: {}", res.message), APP_LOG_CAPACITY);
                    }
                }
                Err(_) => {
                    show_toast("Failed to parse stop response".to_string(), ToastType::Error);
                }
            }
        });
    };

    view! {
        <style>{LOGS_CSS}</style>
        <main class="container">
            <Toast toasts=toasts remove_toast=remove_toast />
            
            <div class="sidebar">
                <div class="header">
                    <h1>"RustFS Launcher"</h1>
                    <p class="subtitle">"Simple launcher for RustFS project"</p>
                    <div class="service-indicator" class:online=move || service_status.get()>
                        <span class="status-dot"></span>
                        <span class="status-text">
                            {move || if service_status.get() { "Service Online" } else { "Service Offline" }}
                        </span>
                    </div>
                </div>

                <ConfigForm
                    config=config
                    set_config=set_config
                    is_running=is_running
                    on_launch=Callback::new(launch_rustfs)
                    on_stop=Callback::new(stop_rustfs)
                />
            </div>

            <div class="logs-section">
                <LogViewer
                    app_logs=app_logs
                    set_app_logs=set_app_logs
                    rustfs_logs=rustfs_logs
                    set_rustfs_logs=set_rustfs_logs
                    current_log_type=current_log_type
                    set_current_log_type=set_current_log_type
                />
            </div>
        </main>
    }
}
