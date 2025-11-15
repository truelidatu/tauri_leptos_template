use leptos::web_sys::console;
use leptos::web_sys::window;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn navigate_back() {
    if let Some(window) = window() {
        if let Ok(history) = window.history() {
            let _ = history.back();
        }
    }
}

#[wasm_bindgen]
pub fn get_url_pathname() -> Result<String, JsValue> {
    let window = window().ok_or("no global `window` exists")?;
    let location = window.location();
    let pathname = location.pathname()?;
    Ok(pathname)
}

#[wasm_bindgen]
pub fn get_dialog_by_id(id: &str) -> Result<leptos::web_sys::HtmlDialogElement, JsValue> {
    let window = window().ok_or("no global `window` exists")?;
    let document = window.document().ok_or("no document")?;
    let element = document.get_element_by_id(id).ok_or("no element")?;
    // document.body().unwrap().style().set_property("overflow", "hidden")?;
    let dialog = element.dyn_into::<leptos::web_sys::HtmlDialogElement>()?;
    Ok(dialog)
}

#[wasm_bindgen]
pub fn log_info(message: &str) {
    console::info_1(&JsValue::from_str(message));
}

#[wasm_bindgen]
pub fn log_warn(message: &str) {
    console::warn_1(&JsValue::from_str(message));
}

#[wasm_bindgen]
pub fn log_error(message: &str) {
    console::error_1(&JsValue::from_str(message));
}

#[wasm_bindgen]
pub fn log_debug(message: &str) {
    console::debug_1(&JsValue::from_str(message));
}

#[wasm_bindgen]
pub fn log_with_multiple_arguments(message: &str, value: i32) {
    console::log_2(
        &JsValue::from_str(message),
        &JsValue::from_f64(value as f64),
    );
}