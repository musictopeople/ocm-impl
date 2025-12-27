use crate::persistence::database::Database;
use crate::core::models::SignedMemory;
use crate::identity::plc::OcmProtocol;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub message_type: MessageType,
    pub payload: String,
    pub from_peer: String,
    pub timestamp: String,
    pub nonce: String,        // Unique nonce for replay protection
    pub hmac: String,         // HMAC for message authentication
}

// Constants for message security and rate limiting
const MAX_MESSAGE_SIZE: usize = 1024 * 1024; // 1MB max message size
const MESSAGE_TIMEOUT_SECS: u64 = 300; // 5 minutes
const NETWORK_SHARED_SECRET: &[u8] = b"ocm-network-secret-change-in-production"; // TODO: Use proper key exchange

// Rate limiting constants
const MAX_MESSAGES_PER_MINUTE: u32 = 60;
const MAX_CONNECTIONS_PER_IP: u32 = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Handshake,
    MemorySync,
    MemoryRequest,
    PeerDiscovery,
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub port: u16,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub did: Option<String>,
}

pub struct OcmNetworking {
    pub local_peer_id: String,
    pub port: u16,
    pub peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    pub ocm_protocol: Arc<Mutex<OcmProtocol>>,
    pub database: Arc<Database>,
    pub message_sender: mpsc::UnboundedSender<NetworkMessage>,
    pub message_receiver: Arc<Mutex<mpsc::UnboundedReceiver<NetworkMessage>>>,
    message_nonces: Arc<Mutex<HashMap<String, u64>>>, // nonce -> timestamp for replay protection
    rate_limiter: Arc<Mutex<RateLimiter>>, // Rate limiting per IP
    connection_tracker: Arc<Mutex<HashMap<String, u32>>>, // IP -> active connection count
}

#[derive(Debug)]
pub struct RateLimiter {
    message_counts: HashMap<String, MessageCount>, // IP -> message count
}

#[derive(Debug)]
struct MessageCount {
    count: u32,
    window_start: u64,
}

// RAII guard for connection cleanup
struct ConnectionGuard {
    connection_tracker: Arc<Mutex<HashMap<String, u32>>>,
    peer_ip: String,
}

impl ConnectionGuard {
    fn new(connection_tracker: Arc<Mutex<HashMap<String, u32>>>, peer_ip: String) -> Self {
        ConnectionGuard {
            connection_tracker,
            peer_ip,
        }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let connection_tracker = self.connection_tracker.clone();
        let peer_ip = self.peer_ip.clone();
        tokio::spawn(async move {
            let mut tracker = connection_tracker.lock().await;
            if let Some(count) = tracker.get_mut(&peer_ip) {
                if *count > 0 {
                    *count -= 1;
                }
                if *count == 0 {
                    tracker.remove(&peer_ip);
                }
            }
        });
    }
}

impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            message_counts: HashMap::new(),
        }
    }
    
    fn check_rate_limit(&mut self, peer_ip: &str) -> Result<(), String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Clean up expired windows
        self.cleanup_expired_windows(now);
        
        // Get or create message count for this IP
        let message_count = self.message_counts
            .entry(peer_ip.to_string())
            .or_insert(MessageCount {
                count: 0,
                window_start: now,
            });
        
        // Check if we're in a new window
        if now - message_count.window_start >= RATE_LIMIT_WINDOW_SECS {
            message_count.count = 0;
            message_count.window_start = now;
        }
        
        // Check rate limit
        if message_count.count >= MAX_MESSAGES_PER_MINUTE {
            return Err(format!("Rate limit exceeded for IP: {}", peer_ip));
        }
        
        // Increment count
        message_count.count += 1;
        Ok(())
    }
    
    fn cleanup_expired_windows(&mut self, now: u64) {
        self.message_counts.retain(|_, count| {
            now - count.window_start < RATE_LIMIT_WINDOW_SECS
        });
    }
}

impl OcmNetworking {
    pub fn new(port: u16, ocm_protocol: OcmProtocol, database: Arc<Database>) -> Self {
        let local_peer_id = uuid::Uuid::new_v4().to_string();
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        OcmNetworking {
            local_peer_id,
            port,
            peers: Arc::new(Mutex::new(HashMap::new())),
            ocm_protocol: Arc::new(Mutex::new(ocm_protocol)),
            database,
            message_sender,
            message_receiver: Arc::new(Mutex::new(message_receiver)),
            message_nonces: Arc::new(Mutex::new(HashMap::new())),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new())),
            connection_tracker: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Input validation methods
    fn validate_message(&self, message: &NetworkMessage) -> Result<(), String> {
        // Validate peer_id format (UUID)
        if uuid::Uuid::parse_str(&message.from_peer).is_err() {
            return Err("Invalid peer ID format".to_string());
        }
        
        // Validate payload size
        if message.payload.len() > MAX_MESSAGE_SIZE {
            return Err(format!("Payload too large: {} bytes", message.payload.len()));
        }
        
        // Validate timestamp format
        if chrono::DateTime::parse_from_rfc3339(&message.timestamp).is_err() {
            return Err("Invalid timestamp format".to_string());
        }
        
        // Validate nonce format (base64)
        if base64::engine::general_purpose::STANDARD.decode(&message.nonce).is_err() {
            return Err("Invalid nonce format".to_string());
        }
        
        // Validate HMAC format
        if base64::engine::general_purpose::STANDARD.decode(&message.hmac).is_err() {
            return Err("Invalid HMAC format".to_string());
        }
        
        Ok(())
    }

    async fn check_connection_limit(&self, peer_ip: &str) -> Result<(), String> {
        let mut tracker = self.connection_tracker.lock().await;
        let current_connections = *tracker.get(peer_ip).unwrap_or(&0);
        
        if current_connections >= MAX_CONNECTIONS_PER_IP {
            return Err(format!("Connection limit exceeded for IP: {}", peer_ip));
        }
        
        // Increment connection count
        tracker.insert(peer_ip.to_string(), current_connections + 1);
        Ok(())
    }

    async fn release_connection(&self, peer_ip: &str) {
        let mut tracker = self.connection_tracker.lock().await;
        if let Some(count) = tracker.get_mut(peer_ip) {
            if *count > 0 {
                *count -= 1;
            }
            if *count == 0 {
                tracker.remove(peer_ip);
            }
        }
    }

    async fn check_rate_limit(&self, peer_ip: &str) -> Result<(), String> {
        let mut rate_limiter = self.rate_limiter.lock().await;
        rate_limiter.check_rate_limit(peer_ip)
    }

    // Message authentication methods
    pub fn create_authenticated_message(
        message_type: MessageType,
        payload: String,
        from_peer: String,
    ) -> NetworkMessage {
        use rand::RngCore;
        let mut rng = rand::rngs::OsRng;
        let mut nonce_bytes = [0u8; 16];
        rng.fill_bytes(&mut nonce_bytes);
        let nonce = general_purpose::STANDARD.encode(&nonce_bytes);
        
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        let mut message = NetworkMessage {
            message_type,
            payload,
            from_peer,
            timestamp,
            nonce,
            hmac: String::new(), // Will be calculated below
        };
        
        // Calculate HMAC over the message content (excluding the hmac field)
        let message_content = Self::get_message_content_for_hmac(&message);
        let mut mac = HmacSha256::new_from_slice(NETWORK_SHARED_SECRET)
            .expect("HMAC can take key of any size");
        mac.update(message_content.as_bytes());
        let hmac_result = mac.finalize();
        message.hmac = general_purpose::STANDARD.encode(hmac_result.into_bytes());
        
        message
    }

    fn get_message_content_for_hmac(message: &NetworkMessage) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            serde_json::to_string(&message.message_type).unwrap_or_default(),
            message.payload,
            message.from_peer,
            message.timestamp,
            message.nonce
        )
    }

    fn verify_message_authentication(&self, message: &NetworkMessage) -> Result<bool, Box<dyn std::error::Error>> {
        // Check message timestamp (prevent old message replay)
        let message_time = chrono::DateTime::parse_from_rfc3339(&message.timestamp)?;
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(message_time.with_timezone(&chrono::Utc));
        
        if age.num_seconds() > MESSAGE_TIMEOUT_SECS as i64 {
            return Ok(false); // Message too old
        }
        
        // Calculate expected HMAC
        let message_content = Self::get_message_content_for_hmac(message);
        let mut mac = HmacSha256::new_from_slice(NETWORK_SHARED_SECRET)
            .expect("HMAC can take key of any size");
        mac.update(message_content.as_bytes());
        let expected_hmac = general_purpose::STANDARD.encode(mac.finalize().into_bytes());
        
        // Constant-time comparison to prevent timing attacks
        if message.hmac.len() != expected_hmac.len() {
            return Ok(false);
        }
        
        let mut result = 0u8;
        for (a, b) in message.hmac.bytes().zip(expected_hmac.bytes()) {
            result |= a ^ b;
        }
        
        Ok(result == 0)
    }

    async fn check_replay_protection(&self, message: &NetworkMessage) -> bool {
        let mut nonces = self.message_nonces.lock().await;
        
        // Clean up old nonces (older than timeout)
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        nonces.retain(|_, &mut timestamp| (now - timestamp) < MESSAGE_TIMEOUT_SECS);
        
        // Check if nonce already exists
        if nonces.contains_key(&message.nonce) {
            return false; // Replay attack detected
        }
        
        // Add nonce
        nonces.insert(message.nonce.clone(), now);
        true
    }

    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("OCM node listening on: {}", addr);

        let _peers = self.peers.clone();
        let _ocm_protocol = self.ocm_protocol.clone();
        let _database = self.database.clone();
        let _message_sender = self.message_sender.clone();
        let self_clone = Arc::new(Self {
            local_peer_id: self.local_peer_id.clone(),
            port: self.port,
            peers: self.peers.clone(),
            ocm_protocol: self.ocm_protocol.clone(),
            database: self.database.clone(),
            message_sender: self.message_sender.clone(),
            message_receiver: self.message_receiver.clone(),
            message_nonces: self.message_nonces.clone(),
            rate_limiter: self.rate_limiter.clone(),
            connection_tracker: self.connection_tracker.clone(),
        });

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        let self_for_task = self_clone.clone();

                        tokio::spawn(async move {
                            if let Err(e) = self_for_task.handle_connection(
                                stream,
                                addr.to_string(),
                            )
                            .await
                            {
                                eprintln!("Error handling connection: {}", e);
                            }
                        });
                    }
                    Err(e) => eprintln!("Failed to accept connection: {}", e),
                }
            }
        });

        Ok(())
    }

    async fn handle_connection(
        &self,
        mut stream: TcpStream,
        peer_addr: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract IP from peer_addr (remove port)
        let peer_ip = peer_addr.split(':').next().unwrap_or(&peer_addr).to_string();
        
        // Check connection limit
        if let Err(e) = self.check_connection_limit(&peer_ip).await {
            eprintln!("Connection rejected: {}", e);
            return Err(e.into());
        }
        
        // Ensure connection cleanup on drop
        let _connection_guard = ConnectionGuard::new(self.connection_tracker.clone(), peer_ip.clone());
        // Use dynamic buffer with size limit
        let mut buffer = Vec::new();
        let mut length_bytes = [0u8; 4];

        loop {
            // Read message length first (4-byte big-endian)
            if stream.read_exact(&mut length_bytes).await.is_err() {
                break; // Connection closed
            }
            
            let message_length = u32::from_be_bytes(length_bytes) as usize;
            
            // Enforce message size limit
            if message_length > MAX_MESSAGE_SIZE {
                eprintln!("Message too large: {} bytes (max: {})", message_length, MAX_MESSAGE_SIZE);
                break;
            }
            
            // Read the actual message
            buffer.resize(message_length, 0);
            stream.read_exact(&mut buffer).await?;

            if let Ok(message) = serde_json::from_slice::<NetworkMessage>(&buffer) {
                // Check rate limit before processing
                if let Err(e) = self.check_rate_limit(&peer_ip).await {
                    eprintln!("Rate limit exceeded: {}", e);
                    continue;
                }
                
                // Validate message format
                if let Err(e) = self.validate_message(&message) {
                    eprintln!("Message validation failed: {}", e);
                    continue;
                }
                
                // Verify message authentication
                if !self.verify_message_authentication(&message)? {
                    eprintln!("Message authentication failed from: {}", peer_addr);
                    continue;
                }
                
                // Check replay protection
                if !self.check_replay_protection(&message).await {
                    eprintln!("Replay attack detected from: {}", peer_addr);
                    continue;
                }
                
                self.process_message(message, &peer_addr).await?;

                // Send authenticated acknowledgment
                let ack = Self::create_authenticated_message(
                    MessageType::Pong,
                    "ack".to_string(),
                    self.local_peer_id.clone(),
                );
                let ack_data = serde_json::to_vec(&ack)?;
                let ack_length = (ack_data.len() as u32).to_be_bytes();
                stream.write_all(&ack_length).await?;
                stream.write_all(&ack_data).await?;
            }
        }

        Ok(())
    }

    async fn process_message(
        &self,
        message: NetworkMessage,
        peer_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match message.message_type {
            MessageType::Handshake => {
                let peer_info = PeerInfo {
                    peer_id: message.from_peer.clone(),
                    address: peer_addr.to_string(),
                    port: 0, // Will be updated from handshake payload
                    last_seen: chrono::Utc::now(),
                    did: None,
                };
                self.peers
                    .lock()
                    .await
                    .insert(message.from_peer.clone(), peer_info);
                println!("Handshake received from peer: {}", message.from_peer);
            }

            MessageType::MemorySync => {
                if let Ok(memory) = serde_json::from_str::<SignedMemory>(&message.payload) {
                    let mut ocm = self.ocm_protocol.lock().await;
                    match ocm.verify_federated_memory(&memory).await {
                        Ok(true) => {
                            if let Err(e) = self.database.create_signed_memory(&memory) {
                                eprintln!("Failed to store federated memory: {}", e);
                            } else {
                                println!(
                                    "✅ Stored federated memory from peer: {}",
                                    message.from_peer
                                );
                            }
                        }
                        Ok(false) => {
                            println!(
                                "❌ Invalid memory signature from peer: {}",
                                message.from_peer
                            );
                        }
                        Err(e) => {
                            eprintln!("Error verifying memory: {}", e);
                        }
                    }
                }
            }

            MessageType::MemoryRequest => {
                // Send our recent memories to the requesting peer via direct connection
                if let Ok(memories) = self.database.list_signed_memories() {
                    // Find the requesting peer info
                    let requesting_peer = {
                        let peers = self.peers.lock().await;
                        peers.get(&message.from_peer).cloned()
                    };
                    
                    if let Some(peer_info) = requesting_peer {
                        for memory in memories.iter().take(10) {
                            // Send last 10 memories directly to requesting peer
                            let sync_message = Self::create_authenticated_message(
                                MessageType::MemorySync,
                                serde_json::to_string(memory)?,
                                self.local_peer_id.clone(),
                            );
                            if let Err(e) = self.send_message_to_peer(&peer_info, &sync_message).await {
                                eprintln!("Failed to send memory to requesting peer: {}", e);
                                break; // Stop sending if connection fails
                            }
                        }
                    }
                }
            }

            MessageType::PeerDiscovery => {
                // Share known peers with requesting peer via direct connection
                let peers_lock = self.peers.lock().await;
                let peer_list: Vec<&PeerInfo> = peers_lock.values().collect();
                let requesting_peer = peers_lock.get(&message.from_peer).cloned();
                
                if let (Some(peer_info), Ok(payload)) = (requesting_peer, serde_json::to_string(&peer_list)) {
                    let discovery_message = Self::create_authenticated_message(
                        MessageType::PeerDiscovery,
                        payload,
                        self.local_peer_id.clone(),
                    );
                    
                    // Send response directly to requesting peer
                    drop(peers_lock);
                    if let Err(e) = self.send_message_to_peer(&peer_info, &discovery_message).await {
                        eprintln!("Failed to send peer discovery response: {}", e);
                    }
                }
            }

            MessageType::Ping => {
                // Update peer's last_seen timestamp
                if let Some(peer) = self.peers.lock().await.get_mut(&message.from_peer) {
                    peer.last_seen = chrono::Utc::now();
                }
            }

            MessageType::Pong => {
                // Connection acknowledged
            }
        }

        Ok(())
    }

    pub async fn connect_to_peer(
        &self,
        peer_addr: &str,
        peer_port: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", peer_addr, peer_port);
        let mut stream = TcpStream::connect(&addr).await?;

        // Send handshake
        let handshake = Self::create_authenticated_message(
            MessageType::Handshake,
            format!(
                "{{\"peer_id\": \"{}\", \"port\": {}}}",
                self.local_peer_id, self.port
            ),
            self.local_peer_id.clone(),
        );

        let handshake_data = serde_json::to_vec(&handshake)?;
        let length = (handshake_data.len() as u32).to_be_bytes();
        stream.write_all(&length).await?;
        stream.write_all(&handshake_data).await?;

        // Add peer to our list
        let peer_info = PeerInfo {
            peer_id: format!("{}:{}", peer_addr, peer_port),
            address: peer_addr.to_string(),
            port: peer_port,
            last_seen: chrono::Utc::now(),
            did: None,
        };

        self.peers
            .lock()
            .await
            .insert(peer_info.peer_id.clone(), peer_info);
        println!("Connected to peer: {}:{}", peer_addr, peer_port);

        Ok(())
    }

    pub async fn broadcast_memory(
        &self,
        memory: &SignedMemory,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let message = Self::create_authenticated_message(
            MessageType::MemorySync,
            serde_json::to_string(memory)?,
            self.local_peer_id.clone(),
        );

        let peers = self.peers.lock().await;
        for peer in peers.values() {
            if let Err(e) = self.send_message_to_peer(peer, &message).await {
                eprintln!("Failed to send memory to peer {}: {}", peer.peer_id, e);
            }
        }

        Ok(())
    }

    async fn send_message_to_peer(
        &self,
        peer: &PeerInfo,
        message: &NetworkMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("{}:{}", peer.address, peer.port);
        let mut stream = TcpStream::connect(&addr).await?;

        let message_data = serde_json::to_vec(message)?;
        
        // Send length-prefixed message (same protocol as handle_connection)
        let length = (message_data.len() as u32).to_be_bytes();
        stream.write_all(&length).await?;
        stream.write_all(&message_data).await?;

        // Wait for acknowledgment with timeout
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            async {
                let mut length_bytes = [0u8; 4];
                stream.read_exact(&mut length_bytes).await?;
                let ack_length = u32::from_be_bytes(length_bytes) as usize;
                
                if ack_length > MAX_MESSAGE_SIZE {
                    return Err("Acknowledgment too large".into());
                }
                
                let mut ack_buffer = vec![0; ack_length];
                stream.read_exact(&mut ack_buffer).await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            }
        ).await??;

        Ok(())
    }

    pub async fn request_memories_from_peers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let request_message = Self::create_authenticated_message(
            MessageType::MemoryRequest,
            "".to_string(),
            self.local_peer_id.clone(),
        );

        let peers = self.peers.lock().await;
        for peer in peers.values() {
            if let Err(e) = self.send_message_to_peer(peer, &request_message).await {
                eprintln!(
                    "Failed to request memories from peer {}: {}",
                    peer.peer_id, e
                );
            }
        }

        Ok(())
    }

    pub async fn discover_peers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let discovery_message = Self::create_authenticated_message(
            MessageType::PeerDiscovery,
            "".to_string(),
            self.local_peer_id.clone(),
        );

        let peers = self.peers.lock().await;
        for peer in peers.values() {
            if let Err(e) = self.send_message_to_peer(peer, &discovery_message).await {
                eprintln!("Failed to discover peers from {}: {}", peer.peer_id, e);
            }
        }

        Ok(())
    }

    pub async fn start_heartbeat(&self) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.peers.clone();
        let local_peer_id = self.local_peer_id.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));

            loop {
                interval.tick().await;

                let _ping_message = Self::create_authenticated_message(
                    MessageType::Ping,
                    "ping".to_string(),
                    local_peer_id.clone(),
                );

                let peers_lock = peers.lock().await;
                for peer in peers_lock.values() {
                    // Send ping (implementation would be similar to send_message_to_peer)
                    println!("Sending heartbeat to peer: {}", peer.peer_id);
                }
            }
        });

        Ok(())
    }
}
