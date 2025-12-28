use wasm_bindgen::prelude::*;
use web_sys::console;

// Import core OCM functionality
use ocm_core::{PlcIdentity, SignedMemory};

mod crypto;
mod storage;
mod utils;
mod websocket;

pub use crypto::*;
pub use storage::*;
pub use utils::*;
pub use websocket::*;

// WeeAlloc removed as it's outdated and causes issues

// This is like the `extern` block from the previous example
#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

// Export a `greet` function from Rust to JavaScript, that alerts a hello message
#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging
macro_rules! log {
    ( $( $t:tt )* ) => {
        console::log_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    // Initialize panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    log!("OCM WASM module loaded!");
}

// OCM-specific WASM exports
#[wasm_bindgen]
pub struct OcmWasm {
    storage: BrowserStorage,
    identity: Option<PlcIdentity>,
    websocket: Option<OcmWebSocket>,
}

#[wasm_bindgen]
impl OcmWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        utils::set_panic_hook();

        Self {
            storage: BrowserStorage::new(),
            identity: None,
            websocket: None,
        }
    }

    #[wasm_bindgen]
    pub fn create_identity(&mut self, handle: Option<String>) -> Result<String, String> {
        let identity = PlcIdentity::generate(handle).map_err(|e| e.to_string())?;

        let did = identity.did.clone();
        self.identity = Some(identity);

        log!("Created identity with DID: {}", did);
        Ok(did)
    }

    #[wasm_bindgen]
    pub async fn init_storage(&mut self) -> Result<(), String> {
        self.storage.init().await.map_err(|e| e.to_string())
    }

    #[wasm_bindgen]
    pub async fn store_memory(&mut self, memory_type: &str, data: &str) -> Result<String, String> {
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| "No identity created".to_string())?;

        let mut memory = SignedMemory::new(&identity.did, memory_type, data);

        // Sign the memory with the identity
        identity
            .sign_memory(&mut memory)
            .map_err(|e| e.to_string())?;

        let memory_id = memory.id.clone();

        // Store in browser storage
        self.storage
            .store_memory(&memory)
            .await
            .map_err(|e| format!("Storage error: {:?}", e))?;

        log!("Stored memory: {}", memory_id);
        Ok(memory_id)
    }

    #[wasm_bindgen]
    pub async fn list_memories(&self) -> Result<String, String> {
        let memories = self
            .storage
            .list_memories()
            .await
            .map_err(|e| format!("Storage error: {:?}", e))?;

        serde_json::to_string(&memories).map_err(|e| e.to_string())
    }

    // WebSocket methods
    #[wasm_bindgen]
    pub fn connect_relay(&mut self, relay_url: &str) -> Result<(), String> {
        let mut ws = OcmWebSocket::new();
        ws.connect(relay_url)
            .map_err(|e| format!("Connection error: {:?}", e))?;
        self.websocket = Some(ws);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_memory_callback(&mut self, callback: &js_sys::Function) {
        if let Some(ws) = &mut self.websocket {
            ws.set_on_memory_received(callback.clone());
        }
    }

    #[wasm_bindgen]
    pub fn send_memory_to_relay(&self, memory_json: &str) -> Result<(), String> {
        let memory: SignedMemory =
            serde_json::from_str(memory_json).map_err(|e| format!("JSON parse error: {}", e))?;

        if let Some(ws) = &self.websocket {
            ws.send_memory(memory_json)?;
        } else {
            return Err("WebSocket not connected".to_string());
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn disconnect_relay(&mut self) {
        if let Some(ws) = &mut self.websocket {
            ws.disconnect();
        }
        self.websocket = None;
    }

    #[wasm_bindgen]
    pub fn is_relay_connected(&self) -> bool {
        if let Some(ws) = &self.websocket {
            ws.is_connected()
        } else {
            false
        }
    }
}
