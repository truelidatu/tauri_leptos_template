mod websocket;

use std::sync::Arc;
use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// #[tauri::command]
// async fn broadcast_message(app_handle: tauri::AppHandle, message: String) -> Result<(), String> {
//     let ws_server = app_handle.state::<Arc<websocket::WebSocketServer>>();
//     ws_server.broadcast(&message).await;
//     Ok(())
// }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    
    let ws_server = Arc::new(websocket::WebSocketServer::new(rt.handle().clone()));
    let ws_server_clone = ws_server.clone();
    
    rt.spawn(async move {
        let result = ws_server_clone.start(6789).await;
        if let Err(e) = result {
            eprintln!("Failed to start WebSocket server: {}", e);
        }
    });

    // let mut web_service = WebService::new();
    // web_service.start();
    tauri::async_runtime::set(rt.handle().clone());
    let builder = tauri::Builder::default();
    builder
        .plugin(tauri_plugin_opener::init())
        .manage(ws_server)
        .invoke_handler(tauri::generate_handler![greet])
        .setup(move |_app| {

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
