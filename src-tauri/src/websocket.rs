use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

type Clients = Arc<Mutex<HashMap<String, WebSocket>>>;

#[derive(Clone)]
pub struct WebSocketServer {
    clients: Clients,
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let app = axum::Router::new()
            .route("/rpc", axum::routing::any(Self::websocket_handler))
            .with_state(self.clone());

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        println!("WebSocket server listening on ws://127.0.0.1:{}", port);

        axum::serve(listener, app).await?;
        Ok(())
    }

    async fn websocket_handler(
        ws: WebSocketUpgrade,
        State(server): State<WebSocketServer>,
    ) -> Response {
        ws.on_upgrade(move |socket| Self::handle_socket(socket))
    }

    async fn handle_socket(mut socket: WebSocket) {
        let client_id = uuid::Uuid::new_v4().to_string();

        // {
        //     let mut clients = self.clients.lock().unwrap();
        //     clients.insert(client_id.clone(), socket);
        // }

        println!("Client connected: {}", client_id);

        loop {
            if let Some(msg) = socket.recv().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        println!("Received message from {}: {}", client_id, text);

                        // Echo back the message
                        if let Err(e) = socket
                            .send(Message::Text(format!("Echo: {}", text).into()))
                            .await
                        {
                            println!("Failed to send message: {}", e);
                            break;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        println!("Client disconnected: {}", client_id);
                        break;
                    }
                    Err(e) => {
                        println!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        }

        // {
        //     let mut clients = self.clients.lock().unwrap();
        //     clients.remove(&client_id);
        // }
    }

    // pub async fn broadcast(&self, message: &str) {
    //     let clients = self.clients.lock().unwrap().clone();

    //     for (client_id, mut socket) in clients {
    //         if let Err(e) = socket.send(Message::Text(message.to_string())).await {
    //             println!("Failed to send to client {}: {}", client_id, e);
    //         }
    //     }
    // }
}
