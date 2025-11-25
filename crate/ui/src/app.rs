use std::sync::atomic::AtomicUsize;

use futures_util::SinkExt;
use futures_util::StreamExt;
use leptos::leptos_dom::logging::console_log;
use leptos::task::spawn_local;
use leptos::{ev::SubmitEvent, prelude::*};
use serde::{Deserialize, Serialize};
use shared::HelloWorldMultiThreadClient;
use shared::TaskDisplay;
use shared::TaskDisplayService;
use wasm_bindgen::prelude::*;
use ws_stream_wasm::{WsMessage, WsMeta};

use crate::web_utils::log_error;
use crate::web_utils::log_info;

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
                        log_error(&format!("Error sending ws message: {:?}", e));
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
                log_error(&format!("Error sending message: {:?}", e));
            }
        }
    };

    let (calculator, set_calculator) = signal_local::<Option<CalculatorClient>>(None);
    let (hello_world_multi_thread_client, set_hello_world_multi_thread_client) =
        signal_local::<Option<HelloWorldMultiThreadClient>>(None);
    let setup_rpc = move |_| {
        spawn_local(async move {
            let (mut _ws, wsio) = WsMeta::connect("ws://127.0.0.1:6789/rpc", None)
                .await
                .expect_throw("assume the connection succeeds");

            log_info("Websocket connected");
            let transport = crate::rpc_transport::WasmWsTransport::new(wsio);
            let (stub, engine) = grsrpc::Builder::new(transport)
                .with_client::<HelloWorldMultiThreadClient>()
                // .with_client::<CalculatorClient>()
                // .with_client::<CalculatorMultiThreadClient>()
                .with_service::<TaskDisplayService<_>>(TaskDisplayServiceImpl)
                .build();

            // set_calculator.set(Some(stub));
            set_hello_world_multi_thread_client.set(Some(stub));

            spawn_local(async move {
                log_info("RPC engine started");
                if let Err(_e) = engine.await {
                    // Handle error
                    log_error(&format!("Error in RPC engine: {:?}", _e));
                }
                log_info("RPC engine terminated");
            });
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
            if let Some(ref mut calculator) = calc.get_untracked() {
                log_info(&format!("Sending RPC message add: 555, 666"));
                let result = calculator.add(555, 666).await;
                log_info(&format!("Calculator result: {}", result));
            } else {
                log_info(&format!("Calculator not set up"));
            }
        });
    };

    let handle_call_update_task_status = move |_| {
        spawn_local(async move {
            if let Some(ref mut calculator) = calculator.get_untracked() {
                let result = calculator.call_update_task_status().await;
                log_info(&format!("call_update_task_status result: {:?}", result));
            } else {
                log_info(&format!("Calculator not set up"));
            }
        });
    };

    let handle_call_async_with_result = move |_| {
        let msg = rpc_msg_to_send.get_untracked();
        if msg.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Some(ref mut calculator) = calculator.get_untracked() {
                let result = calculator.async_with_result(msg).await;
                log_info(&format!("async_with_result result: {:?}", result));
            } else {
                log_info(&format!("Calculator not set up"));
            }
        });
    };

    let handle_call_multi_thread_async_hello_world = move |_| {
        let msg = rpc_msg_to_send.get_untracked();
        if msg.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Some(ref mut hello_world_multi_thread_client) =
                hello_world_multi_thread_client.get_untracked()
            {
                let result = hello_world_multi_thread_client.async_hello_world(msg).await;
                log_info(&format!("async_hello_world result: {:?}", result));
            } else {
                log_info(&format!("HelloWorldMultiThreadClient not set up"));
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
            <button on:click=handle_call_update_task_status>"Call Update Task Status"</button>
            <button on:click=handle_call_async_with_result>"Call Async With Result"</button>

            <button on:click=handle_call_multi_thread_async_hello_world>"Call MultiThread Async Hello World"</button>
        </main>
    }
}

struct TaskDisplayServiceImpl;

// #[async_trait::async_trait]
impl TaskDisplay for TaskDisplayServiceImpl {
    fn update_task_status(&self, task_id: u32, status: String) -> String {
        log_info(&format!("update_task_status({}, {})", task_id, status));
        "updated".to_string()
    }

    async fn async_fn_test(&self, task_id: u32) -> String {
        log_info(&format!("async_fn_test({})", task_id));
        "async_fn_test completed".to_string()
    }
}

#[derive(Clone)]
struct CalculatorClient {
    request_tx: grsrpc::futures_channel::mpsc::UnboundedSender<(usize, CalculatorRequest)>,
    abort_tx: grsrpc::futures_channel::mpsc::UnboundedSender<usize>,
    callback_map: std::rc::Rc<std::cell::RefCell<grsrpc::client::CallbackMap<CalculatorResponse>>>,
    task_set_handle: grsrpc::task_set::TaskSetHandle<()>,
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
        self.request_tx
            .unbounded_send((__seq_id, __request))
            .unwrap();

        let __response_future = grsrpc::futures_util::FutureExt::map(__response_rx, |response| {
            let response = response.unwrap();
            let CalculatorResponse::Add(__inner) = response else {
                panic!("Unexpected response type");
            };
            __inner
        });
        let __abort_tx = self.abort_tx.clone();
        grsrpc::client::RequestFuture::new(
            __response_future,
            std::boxed::Box::new(move || (__abort_tx).unbounded_send(__seq_id).unwrap()),
        )
    }

    pub fn call_update_task_status(&mut self) -> grsrpc::client::RequestFuture<()> {
        let __seq_id = self.seq_id.replace_with(|seq_id| seq_id.wrapping_add(1));
        let __request = CalculatorRequest::CallUpdateTaskStatus;

        let (__response_tx, __response_rx) = grsrpc::futures_channel::oneshot::channel();
        self.callback_map
            .borrow_mut()
            .insert(__seq_id, __response_tx);
        // TODO: handle error
        self.request_tx
            .unbounded_send((__seq_id, __request))
            .unwrap();

        let __response_future = grsrpc::futures_util::FutureExt::map(__response_rx, |response| {
            let response = response.unwrap();
            let CalculatorResponse::CallUpdateTaskStatus(__inner) = response else {
                panic!("Unexpected response type");
            };
            __inner
        });
        let __abort_tx = self.abort_tx.clone();
        grsrpc::client::RequestFuture::new(
            __response_future,
            std::boxed::Box::new(move || (__abort_tx).unbounded_send(__seq_id).unwrap()),
        )
    }

    pub fn async_with_result(&mut self, name: String) -> grsrpc::client::RequestFuture<String> {
        let __seq_id = self.seq_id.replace_with(|seq_id| seq_id.wrapping_add(1));
        let __request = CalculatorRequest::AsyncWithResult(name);

        let (__response_tx, __response_rx) = grsrpc::futures_channel::oneshot::channel();
        self.callback_map
            .borrow_mut()
            .insert(__seq_id, __response_tx);
        // TODO: handle error
        self.request_tx
            .unbounded_send((__seq_id, __request))
            .unwrap();

        let __response_future = grsrpc::futures_util::FutureExt::map(__response_rx, |response| {
            let response = response.unwrap();
            let CalculatorResponse::AsyncWithResult(__inner) = response else {
                panic!("Unexpected response type");
            };
            __inner
        });
        let __abort_tx = self.abort_tx.clone();
        grsrpc::client::RequestFuture::new(
            __response_future,
            std::boxed::Box::new(move || (__abort_tx).unbounded_send(__seq_id).unwrap()),
        )
    }
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorRequest {
    Add { a: i32, b: i32 },
    CallUpdateTaskStatus,
    AsyncWithResult(String),
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorResponse {
    Add(i32),
    CallUpdateTaskStatus(()),
    AsyncWithResult(String),
}

impl From<grsrpc::client::Configuration<CalculatorRequest, CalculatorResponse>>
    for CalculatorClient
{
    fn from(
        (callback_map, request_tx, abort_tx, task_set_handle): grsrpc::client::Configuration<
            CalculatorRequest,
            CalculatorResponse,
        >,
    ) -> Self {
        Self {
            callback_map,
            request_tx,
            abort_tx,
            task_set_handle,
            seq_id: std::default::Default::default(),
        }
    }
}

#[derive(Clone)]
struct CalculatorMultiThreadClient {
    // seq_id: std::sync::Arc<AtomicUsize>,
    // task_set_handle: grsrpc::task_set::TaskSetHandle<()>,
    relay_tx: grsrpc::futures_channel::mpsc::UnboundedSender<(
        grsrpc::futures_channel::oneshot::Sender<CalculatorResponse>,
        grsrpc::futures_channel::oneshot::Receiver<()>,
        CalculatorRequest,
    )>,
}

impl CalculatorMultiThreadClient {
    async fn client_relay_actor(
        mut relay_rx: grsrpc::futures_channel::mpsc::UnboundedReceiver<(
            grsrpc::futures_channel::oneshot::Sender<CalculatorResponse>,
            grsrpc::futures_channel::oneshot::Receiver<()>,
            CalculatorRequest,
        )>,
        client: CalculatorClient,
        mut task_set_handle: grsrpc::task_set::TaskSetHandle<()>,
    ) -> Result<(), ()> {
        while let Some(request) = relay_rx.next().await {
            let mut local_client_clone = client.clone();

            match request {
                (res_tx, abort_relay_rx, CalculatorRequest::Add { a, b }) => {
                    task_set_handle.add(async move {
                        let res = local_client_clone.add(a, b).await;
                        // relay_tx.unbounded_send(CalculatorResponse::Add(res)).unwrap();
                        res_tx.send(CalculatorResponse::Add(res));
                        Ok(())
                    });
                }
                (res_tx, abort_relay_rx, CalculatorRequest::CallUpdateTaskStatus) => {
                    task_set_handle.add(async move {
                        let res = local_client_clone.call_update_task_status().await;
                        // relay_tx.unbounded_send(CalculatorResponse::CallUpdateTaskStatus(res)).unwrap();
                        res_tx.send(CalculatorResponse::CallUpdateTaskStatus(res));
                        Ok(())
                    });
                }
                (res_tx, abort_relay_rx, CalculatorRequest::AsyncWithResult(name)) => {
                    task_set_handle.add(async move {
                        let res = local_client_clone.async_with_result(name).await;
                        // relay_tx.unbounded_send(CalculatorResponse::AsyncWithResult(res)).unwrap();
                        res_tx.send(CalculatorResponse::AsyncWithResult(res));
                        Ok(())
                    });
                }
            }
        }
        Ok(())
    }
}

impl grsrpc::client::Client for CalculatorMultiThreadClient {
    type Response = CalculatorResponse;
    type Request = CalculatorRequest;
}

impl From<grsrpc::client::Configuration<CalculatorRequest, CalculatorResponse>>
    for CalculatorMultiThreadClient
{
    fn from(
        (callback_map, request_tx, abort_tx, mut task_set_handle): grsrpc::client::Configuration<
            CalculatorRequest,
            CalculatorResponse,
        >,
    ) -> Self {
        let task_set_handle_clone = task_set_handle.clone();
        let local_client = CalculatorClient::from((
            callback_map,
            request_tx,
            abort_tx.clone(),
            task_set_handle_clone,
        ));
        let (relay_tx, relay_rx) = grsrpc::futures_channel::mpsc::unbounded::<(
            grsrpc::futures_channel::oneshot::Sender<CalculatorResponse>,
            grsrpc::futures_channel::oneshot::Receiver<()>,
            CalculatorRequest,
        )>();

        let ask_set_handle_clone = task_set_handle.clone();

        task_set_handle.add(Self::client_relay_actor(
            relay_rx,
            local_client,
            ask_set_handle_clone,
        ));
        Self { relay_tx }
    }
}

impl CalculatorMultiThreadClient {
    pub fn add(&self, a: i32, b: i32) -> grsrpc::client::MultiThreadRequestFuture<i32> {
        let (__res_tx, __res_rx) = grsrpc::futures_channel::oneshot::channel();
        let (__abort_relay_tx, __abort_relay_rx) =
            grsrpc::futures_channel::oneshot::channel::<()>();
        self.relay_tx
            .unbounded_send((__res_tx, __abort_relay_rx, CalculatorRequest::Add { a, b }))
            .unwrap();
        let __response_future = grsrpc::futures_util::FutureExt::map(__res_rx, |response| {
            let response = response.unwrap();
            let CalculatorResponse::Add(__inner) = response else {
                panic!("received incorrect response variant")
            };
            __inner
        });

        grsrpc::client::MultiThreadRequestFuture::new(
            __response_future,
            std::boxed::Box::new(move || __abort_relay_tx.send(()).unwrap()),
        )
    }
    pub async fn call_update_task_status(&self) -> () {
        let (__res_tx, __res_rx) = grsrpc::futures_channel::oneshot::channel();
        let (__abort_relay_tx, __abort_relay_rx) =
            grsrpc::futures_channel::oneshot::channel::<()>();
        self.relay_tx
            .unbounded_send((
                __res_tx,
                __abort_relay_rx,
                CalculatorRequest::CallUpdateTaskStatus {},
            ))
            .unwrap();
        let CalculatorResponse::CallUpdateTaskStatus(res) = __res_rx.await.unwrap() else {
            panic!("Unexpected response type");
        };
        res
    }
}

impl From<CalculatorClient> for CalculatorMultiThreadClient {
    fn from(client: CalculatorClient) -> Self {
        let CalculatorClient {
            callback_map,
            request_tx,
            abort_tx,
            mut task_set_handle,
            ..
        } = client.clone();

        let (relay_tx, relay_rx) = grsrpc::futures_channel::mpsc::unbounded::<(
            grsrpc::futures_channel::oneshot::Sender<CalculatorResponse>,
            grsrpc::futures_channel::oneshot::Receiver<()>,
            CalculatorRequest,
        )>();
        let task_set_handle_clone = task_set_handle.clone();

        task_set_handle.add(Self::client_relay_actor(
            relay_rx,
            client,
            task_set_handle_clone,
        ));
        Self { relay_tx }
    }
}
