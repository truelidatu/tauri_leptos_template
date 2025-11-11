use futures_util::StreamExt;
use leptos::leptos_dom::logging::console_log;
use leptos::task::spawn_local;
use leptos::{ev::SubmitEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use ws_stream_wasm::{WsMessage, WsMeta};
use futures_util::SinkExt;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
struct GreetArgs<'a> {
    name: &'a str,
}

#[component]
pub fn App() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (greet_msg, set_greet_msg) = signal(String::new());
    let (ws_tx, set_ws_tx) = signal_local(None);
    let (msg_to_send, set_msg_to_send) = signal(String::new());

    let update_msg_to_send = move |ev| {
        let v = event_target_value(&ev);
        set_msg_to_send.set(v);
    };

    let update_name = move |ev| {
        let v = event_target_value(&ev);
        set_name.set(v);
    };

    let greet = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let name = name.get_untracked();
            if name.is_empty() {
                return;
            }

            let args = serde_wasm_bindgen::to_value(&GreetArgs { name: &name }).unwrap();
            // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
            let new_msg = invoke("greet", args).await.as_string().unwrap();
            set_greet_msg.set(new_msg);
        });
    };

    let connect_to_websocket = move |_| {
        spawn_local(async move {
            let (mut ws, wsio) = WsMeta::connect("ws://127.0.0.1:6789/rpc", None)
                .await
                .expect_throw("assume the connection succeeds");

            let (mut tx, mut rx) = wsio.split();

            // receive messages from the server
            spawn_local(async move {
                while let Some(msg) = rx.next().await {
                    console_log(&format!("Received message: {:?}", msg));
                }
            });

            // Send coroutine
            let (out_tx, mut out_rx) = local_channel::mpsc::channel();
            set_ws_tx.set(Some(out_tx));

            spawn_local(async move {
                while let Some(msg) = out_rx.next().await {
                    if let Err(e) = tx.send(msg).await {
                        console_log(&format!("Error sending ws message: {:?}", e));
                    }
                }
            });
        });
    };

    let handle_send_message = move |ev: SubmitEvent| {
        ev.prevent_default();
        let msg = msg_to_send.get_untracked();
        if msg.is_empty() {
            return;
        }

        if let Some(tx) = ws_tx.get_untracked() {
            if let Err(e) = tx.send(WsMessage::Text(msg)) {
                console_log(&format!("Error sending message: {:?}", e));
            }
        }
    };

    view! {
        <main class="container">
            <h1>"Tauri + Leptos Template"</h1>

            <form class="row" on:submit=greet>
                <input
                    id="greet-input"
                    placeholder="Enter a name..."
                    on:input=update_name
                />
                <button type="submit">"Greet"</button>
            </form>

            <button on:click=connect_to_websocket>"Connect to WebSocket server"</button>
            <form class="row" on:submit=handle_send_message>
                <input
                    placeholder="Message to send"
                    on:input=update_msg_to_send
                />
                <button type="submit">"Send"</button>
            </form>
            <p>{ move || greet_msg.get() }</p>
        </main>
    }
}
