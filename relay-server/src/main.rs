use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "ocm-relay")]
#[command(about = "OCM WebSocket Relay Server for tab-to-tab synchronization")]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value = "8082")]
    port: u16,
}

type Connections = Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("relay_server=info")
        .init();

    let args = Args::parse();
    let addr = format!("{}:{}", args.host, args.port);

    let listener = TcpListener::bind(&addr).await?;
    info!("OCM Relay Server listening on: {}", addr);

    let connections: Connections = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New connection from: {}", addr);
        let connections = Arc::clone(&connections);

        tokio::spawn(handle_connection(stream, connections, addr.to_string()));
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream, connections: Connections, client_addr: String) {
    let client_id = Uuid::new_v4().to_string();

    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed for {}: {}", client_addr, e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = broadcast::channel(1000);

    // Store connection
    {
        let mut conns = connections.lock().await;
        conns.insert(client_id.clone(), tx.clone());
    }

    info!("Client {} connected ({})", client_id, client_addr);

    // Send initial welcome message
    let welcome = serde_json::json!({
        "type": "welcome",
        "client_id": client_id,
        "message": "Connected to OCM relay server"
    });

    if let Err(e) = ws_sender.send(Message::Text(welcome.to_string())).await {
        warn!("Failed to send welcome message to {}: {}", client_id, e);
    }

    // Handle outgoing messages (broadcasts to this client)
    let client_id_clone = client_id.clone();
    let ws_sender_arc = Arc::new(Mutex::new(ws_sender));
    let ws_sender_clone = ws_sender_arc.clone();
    tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            let mut sender = ws_sender_clone.lock().await;
            if let Err(e) = sender.send(Message::Text(message)).await {
                warn!("Failed to send message to {}: {}", client_id_clone, e);
                break;
            }
        }
    });

    // Handle incoming messages from this client
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                info!("Received message from {}: {}", client_id, text);

                // Try to parse as JSON to determine message type
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                        match msg_type {
                            "memory_sync" => {
                                // Broadcast memory to all other clients
                                broadcast_to_others(&connections, &client_id, &text).await;
                            }
                            "ping" => {
                                // Respond with pong
                                let pong = serde_json::json!({
                                    "type": "pong",
                                    "timestamp": chrono::Utc::now().to_rfc3339()
                                });

                                let mut sender = ws_sender_arc.lock().await;
                                if let Err(e) = sender.send(Message::Text(pong.to_string())).await {
                                    warn!("Failed to send pong to {}: {}", client_id, e);
                                }
                            }
                            _ => {
                                // Forward unknown message types to all clients
                                broadcast_to_others(&connections, &client_id, &text).await;
                            }
                        }
                    }
                } else {
                    // Forward non-JSON messages as-is
                    broadcast_to_others(&connections, &client_id, &text).await;
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client {} disconnected", client_id);
                break;
            }
            Ok(Message::Ping(ping)) => {
                let mut sender = ws_sender_arc.lock().await;
                if let Err(e) = sender.send(Message::Pong(ping)).await {
                    warn!("Failed to send pong to {}: {}", client_id, e);
                }
            }
            Ok(_) => {
                // Handle other message types (Binary, Pong, etc.)
            }
            Err(e) => {
                error!("WebSocket error for {}: {}", client_id, e);
                break;
            }
        }
    }

    // Clean up connection
    {
        let mut conns = connections.lock().await;
        conns.remove(&client_id);
    }

    info!("Client {} connection closed", client_id);
}

async fn broadcast_to_others(connections: &Connections, sender_id: &str, message: &str) {
    let conns = connections.lock().await;

    let mut failed_clients = Vec::new();

    for (client_id, tx) in conns.iter() {
        if client_id != sender_id {
            if let Err(_) = tx.send(message.to_string()) {
                failed_clients.push(client_id.clone());
            }
        }
    }

    // Note: Failed clients will be cleaned up when their connections close
    if !failed_clients.is_empty() {
        warn!("Failed to broadcast to {} clients", failed_clients.len());
    }
}
