use futures_util::SinkExt;
use futures_util::StreamExt;
use leptos::leptos_dom::logging::console_log;
use leptos::task::spawn_local;
use leptos::{ev::SubmitEvent, prelude::*};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use ws_stream_wasm::{WsMessage, WsMeta};

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
    let (rpc_msg_to_send, set_rpc_msg_to_send) = signal(String::new());

    let update_msg_to_send = move |ev| {
        let v = event_target_value(&ev);
        set_msg_to_send.set(v);
    };

    let update_rpc_msg_to_send = move |ev| {
        let v = event_target_value(&ev);
        set_rpc_msg_to_send.set(v);
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
            let (mut _ws, wsio) = WsMeta::connect("ws://127.0.0.1:6789/rpc", None)
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

    let (calculator, set_calculator) = signal_local(None);
    let setup_rpc = move |_| {
        spawn_local(async move {
            let (mut _ws, wsio) = WsMeta::connect("ws://127.0.0.1:6789/rpc", None)
                .await
                .expect_throw("assume the connection succeeds");


            let transport = crate::rpc_transport::WasmWsTransport::new(wsio);
            let (stub, service) = grsrpc::Builder::new(transport)
                .with_client::<CalculatorClient>()
                // .with_service::<DisplayService>(DisplayServiceImpl)
                .build();

            set_calculator.set(Some(stub));
            spawn_local(service);
        });
    };

    let handle_rpc_send_message = move |ev: SubmitEvent| {
        ev.prevent_default();
        let msg = rpc_msg_to_send.get_untracked();
        if msg.is_empty() {
            return;
        }

        let calc = calculator;
        spawn_local(async move {
            if let Some(calculator) = calc.get() {
                let mut calc = calculator;
                let result = calc.add(555, 666).await;
                console_log(&format!("Calculator result: {}", result));
            } else {
                console_log(&format!("Calculator not set up"));
            }
        });
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
            <button on:click=setup_rpc>"Setup RPC"</button>
            <form class="row" on:submit=handle_rpc_send_message>
                <input
                    placeholder="Message to send via RPC"
                    on:input=update_rpc_msg_to_send
                />
                <button type="submit">"Send"</button>
            </form>
        </main>
    }
}

#[derive(Clone)]
struct CalculatorClient {
    request_tx: grsrpc::futures_channel::mpsc::UnboundedSender<(usize, CalculatorRequest)>,
    abort_tx: grsrpc::futures_channel::mpsc::UnboundedSender<usize>,
    callback_map: std::rc::Rc<std::cell::RefCell<grsrpc::client::CallbackMap<CalculatorResponse>>>,
    seq_id: std::rc::Rc<std::cell::RefCell<usize>>,
}

impl grsrpc::client::Client for CalculatorClient {
    type Response = CalculatorResponse;
    type Request = CalculatorRequest;
}

impl CalculatorClient {
    pub fn add(&mut self, a: i32, b: i32) -> grsrpc::client::RequestFuture<i32> {
        let __seq_id = self.seq_id.replace_with(|seq_id| seq_id.wrapping_add(1));
        let __request = CalculatorRequest::Add { a, b };

        let (__response_tx, __response_rx) = grsrpc::futures_channel::oneshot::channel();
        self.callback_map
            .borrow_mut()
            .insert(__seq_id, __response_tx);
        // TODO: handle error
        self.request_tx.unbounded_send((__seq_id, __request)).unwrap();

        let __response_future = grsrpc::futures_util::FutureExt::map(__response_rx, |response| {
            let response = response.unwrap();
            match response {
                CalculatorResponse::Add(res) => res,
            }
        });
        let __abort_tx = self.abort_tx.clone();
        let __abort = std::boxed::Box::new(move || (__abort_tx.unbounded_send(__seq_id).unwrap()));
        grsrpc::client::RequestFuture::new(__response_future, __abort)
    }
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorRequest {
    Add { a: i32, b: i32 },
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorResponse {
    Add(i32),
}

impl From<grsrpc::client::Configuration<CalculatorRequest, CalculatorResponse>>
    for CalculatorClient
{
    fn from(
        (callback_map, request_tx, abort_tx): grsrpc::client::Configuration<
            CalculatorRequest,
            CalculatorResponse,
        >,
    ) -> Self {
        Self {
            callback_map,
            request_tx,
            abort_tx,
            seq_id: std::default::Default::default(),
        }
    }
}
