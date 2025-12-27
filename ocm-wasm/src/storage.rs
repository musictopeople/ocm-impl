use js_sys::{Array, Object, Reflect};
use ocm_core::SignedMemory;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;

pub struct BrowserStorage {
    sqlite_ready: bool,
}

impl BrowserStorage {
    pub fn new() -> Self {
        Self {
            sqlite_ready: false,
        }
    }

    pub async fn init(&mut self) -> Result<(), String> {
        // Check if SQLite functions are available (set up by JavaScript)
        let window = web_sys::window().ok_or("No window available")?;

        let sql_execute = js_sys::Reflect::get(&window, &"sqlExecute".into())
            .map_err(|_| "SQLite functions not available")?;

        if sql_execute.is_function() {
            self.sqlite_ready = true;
            crate::log!("✅ SQLite + OPFS storage initialized");
        } else {
            return Err("SQLite functions not properly initialized".to_string());
        }

        Ok(())
    }

    pub async fn store_memory(&mut self, memory: &SignedMemory) -> Result<(), String> {
        if !self.sqlite_ready {
            return Err("SQLite not initialized".to_string());
        }

        // Execute SQL INSERT
        let sql = "INSERT INTO signed_memory (id, did, memory_type, memory_data, content_hash, signature, timestamp, updated_on) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";
        let params = Array::new();
        params.push(&memory.id.clone().into());
        params.push(&memory.did.clone().into());
        params.push(&memory.memory_type.clone().into());
        params.push(&memory.memory_data.clone().into());
        params.push(&memory.content_hash.clone().into());
        params.push(&memory.signature.clone().into());
        params.push(&memory.timestamp.clone().into());
        params.push(&memory.updated_on.clone().into());

        let result = self.call_sql_execute(sql, &params).await?;
        let success = Reflect::get(&result, &"success".into())
            .unwrap()
            .as_bool()
            .unwrap_or(false);
        if !success {
            return Err("Failed to store memory in SQLite".to_string());
        }

        Ok(())
    }

    pub async fn list_memories(&self) -> Result<Vec<SignedMemory>, String> {
        if !self.sqlite_ready {
            return Err("SQLite not initialized".to_string());
        }

        let sql = "SELECT * FROM signed_memory ORDER BY timestamp DESC";
        let params = Array::new();

        let result = self.call_sql_query(sql, &params).await?;
        let success = Reflect::get(&result, &"success".into())
            .unwrap()
            .as_bool()
            .unwrap_or(false);

        if !success {
            return Err("Failed to query memories from SQLite".to_string());
        }

        let data = Reflect::get(&result, &"data".into()).unwrap();
        let data_array: Array = data.dyn_into().map_err(|_| "Invalid data format")?;

        let mut memories = Vec::new();
        for i in 0..data_array.length() {
            let item = data_array.get(i);
            let memory: SignedMemory = serde_wasm_bindgen::from_value(item)
                .map_err(|e| format!("Deserialization error: {:?}", e))?;
            memories.push(memory);
        }

        Ok(memories)
    }

    async fn call_sql_execute(&self, sql: &str, params: &Array) -> Result<Object, String> {
        let window = web_sys::window().ok_or("No window")?;
        let sql_execute = js_sys::Reflect::get(&window, &"sqlExecute".into())
            .map_err(|_| "sqlExecute not found")?;
        let sql_function: js_sys::Function = sql_execute
            .dyn_into()
            .map_err(|_| "sqlExecute not a function")?;

        let args = Array::new();
        args.push(&sql.into());
        args.push(&params.into());

        let result = sql_function
            .apply(&JsValue::NULL, &args)
            .map_err(|e| format!("SQL execute failed: {:?}", e))?;

        let promise: js_sys::Promise = result
            .dyn_into()
            .map_err(|_| "Expected promise from sqlExecute")?;

        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("SQL promise failed: {:?}", e))?;

        result
            .dyn_into()
            .map_err(|_| "Invalid result format".to_string())
    }

    async fn call_sql_query(&self, sql: &str, params: &Array) -> Result<Object, String> {
        let window = web_sys::window().ok_or("No window")?;
        let sql_query =
            js_sys::Reflect::get(&window, &"sqlQuery".into()).map_err(|_| "sqlQuery not found")?;
        let sql_function: js_sys::Function = sql_query
            .dyn_into()
            .map_err(|_| "sqlQuery not a function")?;

        let args = Array::new();
        args.push(&sql.into());
        args.push(&params.into());

        let result = sql_function
            .apply(&JsValue::NULL, &args)
            .map_err(|e| format!("SQL query failed: {:?}", e))?;

        let promise: js_sys::Promise = result
            .dyn_into()
            .map_err(|_| "Expected promise from sqlQuery")?;

        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("SQL promise failed: {:?}", e))?;

        result
            .dyn_into()
            .map_err(|_| "Invalid result format".to_string())
    }
}
