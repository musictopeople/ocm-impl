use crate::networking::protocol::{OcmNetworking, PeerInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryBeacon {
    pub peer_id: String,
    pub did: Option<String>,
    pub port: u16,
    pub capabilities: Vec<String>,
    pub version: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRequest {
    pub requesting_peer_id: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponse {
    pub responding_peer_id: String,
    pub peers: Vec<PeerInfo>,
    pub timestamp: String,
}

pub struct PeerDiscovery {
    pub local_peer_id: String,
    pub discovery_port: u16,
    pub ocm_port: u16,
    pub known_peers: Arc<Mutex<HashMap<String, PeerInfo>>>,
    pub capabilities: Vec<String>,
    pub did: Option<String>,
}

impl PeerDiscovery {
    pub fn new(
        local_peer_id: String,
        discovery_port: u16,
        ocm_port: u16,
        did: Option<String>,
    ) -> Self {
        PeerDiscovery {
            local_peer_id,
            discovery_port,
            ocm_port,
            known_peers: Arc::new(Mutex::new(HashMap::new())),
            capabilities: vec![
                "memory-sync".to_string(),
                "peer-discovery".to_string(),
                "identity-verification".to_string(),
            ],
            did,
        }
    }

    pub async fn start_discovery_service(&self) -> Result<(), Box<dyn std::error::Error>> {
        let discovery_addr = format!("0.0.0.0:{}", self.discovery_port);
        let socket = UdpSocket::bind(&discovery_addr).await?;
        println!("🔍 Peer discovery service listening on: {}", discovery_addr);

        let local_peer_id = self.local_peer_id.clone();
        let ocm_port = self.ocm_port;
        let did = self.did.clone();
        let capabilities = self.capabilities.clone();
        let known_peers = self.known_peers.clone();

        tokio::spawn(async move {
            let mut buffer = [0u8; 1024];

            loop {
                match socket.recv_from(&mut buffer).await {
                    Ok((size, addr)) => {
                        let data = &buffer[..size];

                        if let Ok(beacon) = serde_json::from_slice::<DiscoveryBeacon>(data) {
                            Self::handle_discovery_beacon(beacon, addr.to_string(), &known_peers)
                                .await;
                        } else if let Ok(request) = serde_json::from_slice::<DiscoveryRequest>(data)
                        {
                            Self::handle_discovery_request(
                                request,
                                addr,
                                &socket,
                                &local_peer_id,
                                &did,
                                &capabilities,
                                ocm_port,
                                &known_peers,
                            )
                            .await;
                        }
                    }
                    Err(e) => {
                        eprintln!("Discovery service error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    async fn handle_discovery_beacon(
        beacon: DiscoveryBeacon,
        peer_addr: String,
        known_peers: &Arc<Mutex<HashMap<String, PeerInfo>>>,
    ) {
        // Extract IP address from socket address
        let ip = peer_addr
            .split(':')
            .next()
            .unwrap_or("127.0.0.1")
            .to_string();

        let peer_info = PeerInfo {
            peer_id: beacon.peer_id.clone(),
            address: ip,
            port: beacon.port,
            last_seen: chrono::Utc::now(),
            did: beacon.did.clone(),
        };

        known_peers
            .lock()
            .await
            .insert(beacon.peer_id.clone(), peer_info);
        println!(
            "🔍 Discovered peer: {} at port {}",
            beacon.peer_id, beacon.port
        );
    }

    async fn handle_discovery_request(
        _request: DiscoveryRequest,
        peer_addr: std::net::SocketAddr,
        socket: &UdpSocket,
        local_peer_id: &str,
        did: &Option<String>,
        capabilities: &[String],
        ocm_port: u16,
        known_peers: &Arc<Mutex<HashMap<String, PeerInfo>>>,
    ) {
        // Respond with our beacon and known peers
        let beacon = DiscoveryBeacon {
            peer_id: local_peer_id.to_string(),
            did: did.clone(),
            port: ocm_port,
            capabilities: capabilities.to_vec(),
            version: "0.1.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Ok(beacon_data) = serde_json::to_vec(&beacon) {
            let _ = socket.send_to(&beacon_data, peer_addr).await;
        }

        // Also send known peers as a separate response
        let peers_lock = known_peers.lock().await;
        let peers: Vec<PeerInfo> = peers_lock.values().cloned().collect();
        drop(peers_lock);

        let response = DiscoveryResponse {
            responding_peer_id: local_peer_id.to_string(),
            peers,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Ok(response_data) = serde_json::to_vec(&response) {
            let _ = socket.send_to(&response_data, peer_addr).await;
        }
    }

    pub async fn broadcast_beacon(&self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;

        let beacon = DiscoveryBeacon {
            peer_id: self.local_peer_id.clone(),
            did: self.did.clone(),
            port: self.ocm_port,
            capabilities: self.capabilities.clone(),
            version: "0.1.0".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let beacon_data = serde_json::to_vec(&beacon)?;

        // Broadcast to local network
        let broadcast_addr = format!("255.255.255.255:{}", self.discovery_port);
        socket.send_to(&beacon_data, &broadcast_addr).await?;

        println!("📡 Broadcasted discovery beacon to local network");
        Ok(())
    }

    pub async fn request_peers(&self, target_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;

        let request = DiscoveryRequest {
            requesting_peer_id: self.local_peer_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let request_data = serde_json::to_vec(&request)?;
        let target = format!("{}:{}", target_addr, self.discovery_port);

        socket.send_to(&request_data, &target).await?;
        println!("🔍 Requested peer list from: {}", target);

        Ok(())
    }

    pub async fn start_periodic_discovery(&self) -> Result<(), Box<dyn std::error::Error>> {
        let discovery_port = self.discovery_port;
        let local_peer_id = self.local_peer_id.clone();
        let did = self.did.clone();
        let ocm_port = self.ocm_port;
        let capabilities = self.capabilities.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Create a new discovery instance for broadcasting
                let discovery = PeerDiscovery {
                    local_peer_id: local_peer_id.clone(),
                    discovery_port,
                    ocm_port,
                    known_peers: Arc::new(Mutex::new(HashMap::new())),
                    capabilities: capabilities.clone(),
                    did: did.clone(),
                };

                if let Err(e) = discovery.broadcast_beacon().await {
                    eprintln!("Failed to broadcast discovery beacon: {}", e);
                }

                println!("🔄 Periodic discovery beacon sent");
            }
        });

        Ok(())
    }

    pub async fn get_known_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.lock().await.values().cloned().collect()
    }

    pub async fn add_seed_peers(
        &self,
        seed_addrs: Vec<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for addr in seed_addrs {
            if let Err(e) = self.request_peers(addr).await {
                eprintln!("Failed to contact seed peer {}: {}", addr, e);
            } else {
                println!("🌱 Contacted seed peer: {}", addr);
            }
        }
        Ok(())
    }

    pub async fn connect_discovered_peers(
        &self,
        networking: &OcmNetworking,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peers = self.get_known_peers().await;

        for peer in peers {
            if let Err(e) = networking.connect_to_peer(&peer.address, peer.port).await {
                eprintln!(
                    "Failed to connect to discovered peer {}: {}",
                    peer.peer_id, e
                );
            } else {
                println!("✅ Connected to discovered peer: {}", peer.peer_id);
            }
        }

        Ok(())
    }
}
