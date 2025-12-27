use wasm_bindgen::prelude::*;

// When the `console_error_panic_hook` feature is enabled, we can call the
// `set_panic_hook` function at least once during initialization, and then
// we will get better error messages if our code ever panics.
//
// For more details see
// https://github.com/rustwasm/console_error_panic_hook#readme
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

// A macro to provide `eprintln!(..)`-style syntax for `console.error` logging.
#[macro_export]
macro_rules! error {
    ( $( $t:tt )* ) => {
        web_sys::console::error_1(&format!( $( $t )* ).into());
    }
}

#[wasm_bindgen]
extern "C" {
    // Bind the `alert` function from the browser
    fn alert(s: &str);
    
    // Bind the `confirm` function from the browser
    fn confirm(s: &str) -> bool;
}

#[wasm_bindgen]
pub fn show_alert(message: &str) {
    alert(message);
}

#[wasm_bindgen]
pub fn show_confirm(message: &str) -> bool {
    confirm(message)
}

// Helper function to get current timestamp
#[wasm_bindgen]
pub fn get_timestamp() -> String {
    let date = js_sys::Date::new_0();
    date.to_iso_string().as_string().unwrap()
}

// Helper function to generate UUIDs (using crypto.randomUUID if available)
#[wasm_bindgen]
pub fn generate_uuid() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}