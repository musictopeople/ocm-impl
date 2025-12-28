use ocm_core::SignedMemory;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::*;

#[wasm_bindgen]
pub struct OcmWebSocket {
    ws: Option<WebSocket>,
    on_message_callback: Option<js_sys::Function>,
}

#[wasm_bindgen]
impl OcmWebSocket {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            ws: None,
            on_message_callback: None,
        }
    }

    #[wasm_bindgen]
    pub fn connect(&mut self, relay_url: &str) -> Result<(), JsValue> {
        let ws = WebSocket::new(relay_url)?;

        // Set binary type to handle binary messages (using string for now)
        // ws.set_binary_type(web_sys::BinaryType::Arraybuffer); // BinaryType not available in web-sys

        // Setup event handlers
        let onopen = Closure::wrap(Box::new(move |_event| {
            web_sys::console::log_1(&"WebSocket connected to relay".into());
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let onclose = Closure::wrap(Box::new(move |_event| {
            web_sys::console::log_1(&"WebSocket connection closed".into());
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
        onclose.forget();

        let onerror = Closure::wrap(Box::new(move |event| {
            web_sys::console::error_1(&format!("WebSocket error: {:?}", event).into());
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        self.ws = Some(ws);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_on_memory_received(&mut self, callback: js_sys::Function) {
        if let Some(ws) = &self.ws {
            let callback_clone = callback.clone();
            self.on_message_callback = Some(callback);
            let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Ok(text) = event.data().dyn_into::<js_sys::JsString>() {
                    let text_string = String::from(text);

                    // Parse relay protocol message
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text_string) {
                        if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                            match msg_type {
                                "memory_sync" => {
                                    if let Some(data) = json.get("data") {
                                        if let Ok(memory) =
                                            serde_json::from_value::<SignedMemory>(data.clone())
                                        {
                                            let memory_js =
                                                serde_wasm_bindgen::to_value(&memory).unwrap();
                                            let _ =
                                                callback_clone.call1(&JsValue::NULL, &memory_js);
                                        }
                                    }
                                }
                                "welcome" => {
                                    web_sys::console::log_1(&"Connected to relay server".into());
                                }
                                _ => {
                                    // Handle other message types if needed
                                    web_sys::console::log_1(
                                        &format!("Received message type: {}", msg_type).into(),
                                    );
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);

            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
        }
    }

    #[wasm_bindgen]
    pub fn send_memory(&self, memory_json: &str) -> Result<(), String> {
        if let Some(ws) = &self.ws {
            // Parse the memory JSON first to validate it
            let memory: SignedMemory = serde_json::from_str(memory_json)
                .map_err(|e| format!("Invalid memory JSON: {}", e))?;

            // Wrap memory in the relay protocol format
            let message = serde_json::json!({
                "type": "memory_sync",
                "data": memory
            });

            let json = serde_json::to_string(&message).map_err(|e| e.to_string())?;

            ws.send_with_str(&json)
                .map_err(|e| format!("Send error: {:?}", e))?;
            web_sys::console::log_1(&format!("Sent memory: {}", memory.id).into());
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub fn disconnect(&mut self) {
        if let Some(ws) = &self.ws {
            let _ = ws.close();
        }
        self.ws = None;
    }

    #[wasm_bindgen]
    pub fn is_connected(&self) -> bool {
        if let Some(ws) = &self.ws {
            ws.ready_state() == WebSocket::OPEN
        } else {
            false
        }
    }
}
