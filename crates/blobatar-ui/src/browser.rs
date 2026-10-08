use blobatar_export::Settings;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/browser.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn download(bytes: &[u8], filename: &str, mime: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = chooseSettings)]
    pub fn choose_settings() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_name = copyText)]
    pub fn copy_text(text: &str) -> Result<js_sys::Promise, JsValue>;
}

pub fn save(bytes: &[u8], filename: &str, mime: &str) -> Result<(), String> {
    download(bytes, filename, mime).map_err(error)
}

pub async fn load(promise: Result<js_sys::Promise, JsValue>) -> Result<Option<Settings>, String> {
    let result = wasm_bindgen_futures::JsFuture::from(promise.map_err(error)?)
        .await
        .map_err(error)?;
    if result.is_null() {
        return Ok(None);
    }
    let json = result.as_string().ok_or("Expected a JSON text file")?;
    Settings::from_json(&json).map(Some)
}

pub fn error(error: JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            error
                .dyn_ref::<js_sys::Error>()
                .map(|error| String::from(error.message()))
        })
        .unwrap_or_else(|| "Browser operation failed".into())
}
