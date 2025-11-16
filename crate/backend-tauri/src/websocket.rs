use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{Sink, SinkExt, Stream, StreamExt};
use shared::TaskDisplayClient;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};
use std::{
    collections::HashMap,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

type Clients = Arc<Mutex<HashMap<String, WebSocket>>>;

#[derive(Clone)]
pub struct WebSocketServer {
    clients: Clients,
    // socket_tx: mpsc::UnboundedSender<WebSocket>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let (socket_tx, mut socket_rx) = mpsc::unbounded_channel::<WebSocket>();

        // Start the single-threaded runtime to handle all sockets
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create single-threaded runtime");

            rt.block_on(async {
                let local_set = tokio::task::LocalSet::new();

                local_set
                    .run_until(async {
                        while let Some(socket) = socket_rx.recv().await {
                            tokio::task::spawn_local(async move {
                                Self::handle_socket_in_local_set(socket).await;
                            });
                        }
                    })
                    .await;
            });
        });

        let app = axum::Router::new()
            .route("/rpc", axum::routing::any(Self::websocket_handler))
            .with_state(socket_tx);

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        println!("WebSocket server listening on ws://127.0.0.1:{}", port);

        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn websocket_handler(
        ws: WebSocketUpgrade,
        State(socket_tx): State<mpsc::UnboundedSender<WebSocket>>,
    ) -> Response {
        ws.on_upgrade(move |socket| Self::handle_socket(socket, socket_tx))
    }

    async fn handle_socket(socket: WebSocket, socket_tx: mpsc::UnboundedSender<WebSocket>) {
        // Send the socket to the single-threaded runtime
        if let Err(_) = socket_tx.send(socket) {
            println!("Failed to send socket to handler runtime");
        }
    }

    async fn handle_socket_in_local_set(socket: WebSocket) {
        let client_id = uuid::Uuid::new_v4().to_string();

        println!("Client connected: {}", client_id);

        let transport = WsTransport::new(socket);
        let state = Rc::new(RefCell::new(None));
        let calculator_service_impl = CalculatorServiceImpl::new(state.clone());

        let (task_display_stub, service) = grsrpc::Builder::new(transport)
            .with_client::<TaskDisplayClient>()
            .with_service::<CalculatorService<_>>(calculator_service_impl)
            .build();
        tokio::task::spawn_local(async move {
            println!("CalculatorService started");
            if let Err(e) = service.await {
                eprintln!("CalculatorService finished with error: {:?}", e);
            }
            println!("CalculatorService finished");
        });
        let task_display_stub_clone = task_display_stub.clone();
        *state.borrow_mut() = Some(task_display_stub_clone);
    }
}

struct CalculatorService<T> {
    server_impl: T,
}

impl<T: Calculator> grsrpc::service::Service for CalculatorService<T> {
    type Request = CalculatorRequest;
    type Response = CalculatorResponse;

    fn is_async_request(__request: &Self::Request) -> bool {
        match __request {
            Self::Request::CallUpdateTaskStatus => true,
            Self::Request::Add { a: _, b: _ } => false,
            _ => false,
        }
    }

    fn execute(
        &self,
        __seq_id: usize,
        __request: Self::Request,
    ) -> (usize, Option<Self::Response>) {
        let __result = match __request {
            Self::Request::Add { a, b } => {
                let __response = self.server_impl.add(a, b);
                Some(CalculatorResponse::Add(__response))
            }
            _ => panic!("unexpected request variant, the request handler may be async"),
        };
        (__seq_id, __result)
    }

    async fn execute_async(
        &self,
        __seq_id: usize,
        __abort_rx: grsrpc::futures_channel::oneshot::Receiver<()>,
        __request: Self::Request,
    ) -> (usize, Option<Self::Response>) {
        let __result = match __request {
            Self::Request::CallUpdateTaskStatus => {
                let __response = self.server_impl.call_update_task_status().await;
                Some(CalculatorResponse::CallUpdateTaskStatus(__response))
            }
            _ => panic!("unexpected request variant, the request handler may be sync"),
        };
        (__seq_id, __result)
    }
}

impl<T: Calculator> std::convert::From<T> for CalculatorService<T> {
    fn from(server_impl: T) -> Self {
        Self { server_impl }
    }
}

trait Calculator {
    fn add(&self, a: i32, b: i32) -> i32;
    async fn call_update_task_status(&self) -> ();
}
struct CalculatorServiceImpl {
    task_display_stub: Rc<RefCell<Option<TaskDisplayClient>>>,
}

impl CalculatorServiceImpl {
    pub fn new(task_display_stub: Rc<RefCell<Option<TaskDisplayClient>>>) -> Self {
        Self { task_display_stub }
    }
}

impl Calculator for CalculatorServiceImpl {
    fn add(&self, a: i32, b: i32) -> i32 {
        println!("CalculatorServiceImpl::add({}, {})", a, b);
        a + b
    }
    async fn call_update_task_status(&self) -> () {
        if let Some(task_display_stub) = self.task_display_stub.borrow().as_ref() {
            let res = task_display_stub
                .async_fn_test(333)
                .await;
            println!("call_update_task_status {:?}", res);
        } else {
            println!("task_display_stub is None");
        }
    }
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorRequest {
    Add { a: i32, b: i32 },
    CallUpdateTaskStatus,
}

#[derive(grsrpc::serde::Serialize, grsrpc::serde::Deserialize)]
enum CalculatorResponse {
    Add(i32),
    CallUpdateTaskStatus(()),
}

struct WsTransport(WebSocket);

impl WsTransport {
    pub fn new(stream: WebSocket) -> Self {
        Self(stream)
    }
}

impl Stream for WsTransport {
    type Item = Vec<u8>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_next(cx) {
                // TODO: consider return error if ws message is text. Current implementation will make down stream error when deserialize which is fine
                Poll::Ready(Some(Ok(Message::Text(text)))) => {
                    Poll::Ready(Some(text.as_bytes().to_vec()))
                }
                Poll::Ready(Some(Ok(Message::Binary(bytes)))) => Poll::Ready(Some(bytes.to_vec())),
                Poll::Ready(Some(Err(_e))) => Poll::Ready(None),
                Poll::Ready(Some(Ok(Message::Ping(_)))) => Poll::Pending,
                Poll::Ready(Some(Ok(Message::Pong(_)))) => Poll::Pending,
                Poll::Ready(Some(Ok(Message::Close(_)))) => Poll::Pending,
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

impl Sink<Vec<u8>> for WsTransport {
    type Error = axum::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_ready(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Self::Error> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            ws_stream.start_send(Message::Binary(item.into()))
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_flush(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        unsafe {
            let ws_stream = Pin::new(&mut self.get_unchecked_mut().0);
            match ws_stream.poll_close(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
