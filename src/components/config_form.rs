use crate::types::RustFsConfig;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use serde_json;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    async fn open(options: JsValue) -> JsValue;
}

#[component]
pub fn ConfigForm(
    #[prop(into)] config: Signal<RustFsConfig>,
    #[prop(into)] set_config: WriteSignal<RustFsConfig>,
    #[prop(into)] is_running: Signal<bool>,
    #[prop(into)] on_launch: Callback<SubmitEvent>,
    #[prop(into)] on_stop: Callback<()>,
) -> impl IntoView {
    let (show_secret, set_show_secret) = signal(false);
    let (is_drag_over, set_is_drag_over) = signal(false);

    // Setup Tauri File Drop listener
    Effect::new(move |_| {
        spawn_local(async move {
            if let Some(window) = web_sys::window() {
                if let Ok(tauri) = js_sys::Reflect::get(&window, &"__TAURI__".into()) {
                    if let Ok(event) = js_sys::Reflect::get(&tauri, &"event".into()) {
                        if let Ok(listen) = js_sys::Reflect::get(&event, &"listen".into()) {
                            let listen_fn = js_sys::Function::from(listen);

                            // Handle File Drop
                            let drop_handler = Closure::wrap(Box::new(move |event: JsValue| {
                                set_is_drag_over.set(false);
                                if let Ok(payload) = js_sys::Reflect::get(&event, &"payload".into())
                                {
                                    if let Ok(paths) =
                                        serde_wasm_bindgen::from_value::<Vec<String>>(payload)
                                    {
                                        if let Some(first_path) = paths.first() {
                                            set_config.update(|c| c.data_path = first_path.clone());
                                        }
                                    }
                                }
                            })
                                as Box<dyn FnMut(JsValue)>);

                            // Handle Drag Hover (Enter)
                            let hover_handler = Closure::wrap(Box::new(move |_: JsValue| {
                                set_is_drag_over.set(true);
                            })
                                as Box<dyn FnMut(JsValue)>);

                            // Handle Drag Cancel (Leave)
                            let cancel_handler = Closure::wrap(Box::new(move |_: JsValue| {
                                set_is_drag_over.set(false);
                            })
                                as Box<dyn FnMut(JsValue)>);

                            let _ = listen_fn.call2(
                                &event,
                                &"tauri://file-drop".into(),
                                drop_handler.as_ref().unchecked_ref(),
                            );
                            let _ = listen_fn.call2(
                                &event,
                                &"tauri://file-drop-hover".into(),
                                hover_handler.as_ref().unchecked_ref(),
                            );
                            let _ = listen_fn.call2(
                                &event,
                                &"tauri://file-drop-cancelled".into(),
                                cancel_handler.as_ref().unchecked_ref(),
                            );

                            drop_handler.forget();
                            hover_handler.forget();
                            cancel_handler.forget();
                        }
                    }
                }
            }
        });
    });

    let select_folder = move |_| {
        spawn_local(async move {
            let options = serde_wasm_bindgen::to_value(&serde_json::json!({
                "directory": true,
                "title": "Select RustFS Data Directory"
            }))
            .unwrap();

            if let Some(result) = open(options).await.as_string() {
                if !result.is_empty() {
                    set_config.update(|c| c.data_path = result);
                }
            }
        });
    };

    let (error_message, set_error_message) = signal(Option::<String>::None);

    let handle_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        if is_running.get() {
            // Stop mode
            on_stop.run(());
        } else {
            // Launch mode
            set_error_message.set(None);

            let current_config = config.get();

            if current_config.data_path.trim().is_empty() {
                set_error_message.set(Some("Data path is required".to_string()));
                return;
            }

            if let Some(port) = current_config.port {
                if port == 0 {
                    set_error_message.set(Some("Port must be greater than 0".to_string()));
                    return;
                }
            }

            on_launch.run(ev);
        }
    };

    view! {
        <form class="config-form" on:submit=handle_submit>
            <div class="form-group">
                <label for="data-path">"Data Path" <span class="required">"*"</span></label>
                <div
                    class="path-input-group"
                    class:drag-over=move || is_drag_over.get()
                >
                    <input
                        id="data-path"
                        type="text"
                        placeholder="Select or drop data directory..."
                        prop:value=move || config.get().data_path
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            set_config.update(|c| c.data_path = value.clone());
                            if !value.is_empty() {
                                set_error_message.set(None);
                            }
                        }
                    />
                    <button type="button" class="browse-btn" on:click=select_folder>
                        "Browse"
                    </button>
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label for="port">"Port"</label>
                    <input
                        id="port"
                        type="number"
                        placeholder="8080"
                        min="1"
                        max="65535"
                        prop:value=move || config.get().port.map(|p| p.to_string()).unwrap_or_default()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            let port = if value.is_empty() { None } else { value.parse().ok() };
                            set_config.update(|c| c.port = port);
                        }
                    />
                </div>
                <div class="form-group">
                    <label for="host">"Host"</label>
                    <input
                        id="host"
                        type="text"
                        placeholder="127.0.0.1"
                        prop:value=move || config.get().host.unwrap_or_default()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            let host = if value.is_empty() { None } else { Some(value) };
                            set_config.update(|c| c.host = host);
                        }
                    />
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <label for="access-key">"Access Key"</label>
                    <input
                        id="access-key"
                        type="text"
                        placeholder="rustfsadmin"
                        prop:value=move || config.get().access_key.unwrap_or_default()
                        on:input=move |ev| {
                            let value = event_target_value(&ev);
                            let access_key = if value.is_empty() { None } else { Some(value) };
                            set_config.update(|c| c.access_key = access_key);
                        }
                    />
                </div>
                <div class="form-group">
                    <label for="secret-key">"Secret Key"</label>
                    <div class="input-with-toggle">
                        <input
                            id="secret-key"
                            type=move || if show_secret.get() { "text" } else { "password" }
                            placeholder="rustfsadmin"
                            prop:value=move || config.get().secret_key.unwrap_or_default()
                            on:input=move |ev| {
                                let value = event_target_value(&ev);
                                let secret_key = if value.is_empty() { None } else { Some(value) };
                                set_config.update(|c| c.secret_key = secret_key);
                            }
                        />
                        <button
                            type="button"
                            class="toggle-visibility"
                            on:click=move |_| set_show_secret.update(|show| *show = !*show)
                            title=move || if show_secret.get() { "Hide Secret" } else { "Show Secret" }
                        >
                            {move || if show_secret.get() {
                                view! {
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
                                        <line x1="1" y1="1" x2="23" y2="23"></line>
                                    </svg>
                                }.into_any()
                            } else {
                                view! {
                                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                        <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
                                        <circle cx="12" cy="12" r="3"></circle>
                                    </svg>
                                }.into_any()
                            }}
                        </button>
                    </div>
                </div>
            </div>

            <div class="form-row">
                <div class="form-group">
                    <div class="checkbox-group">
                        <input
                            id="console-enable"
                            type="checkbox"
                            prop:checked=move || config.get().console_enable
                            on:change=move |ev| {
                                let checked = event_target_checked(&ev);
                                set_config.update(|c| c.console_enable = checked);
                            }
                        />
                        <label for="console-enable">"Enable Console"</label>
                    </div>
                </div>
            </div>

            <div class="form-actions">
                <button
                    type="submit"
                    class="launch-btn"
                    class:stop-btn=move || is_running.get()
                    disabled=move || !is_running.get() && config.get().data_path.is_empty()
                >
                    { move || if is_running.get() { "Stop RustFS" } else { "Launch RustFS" } }
                </button>
                <Show when=move || error_message.get().is_some()>
                    <div class="error-message">
                        { move || error_message.get() }
                    </div>
                </Show>
            </div>
        </form>
    }
}
