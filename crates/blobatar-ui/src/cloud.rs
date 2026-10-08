use blobatar_export::Settings;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

pub const PAGE_SIZE: usize = 12;

#[derive(Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Deserialize)]
pub struct Work {
    pub id: String,
    pub settings: Settings,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct Page {
    pub user: User,
    pub rows: Vec<Work>,
    pub more: bool,
}

#[derive(Deserialize)]
pub struct Saved {
    pub user: User,
    pub row: Work,
}

#[derive(Default)]
pub struct Wall {
    pub open: bool,
    pub configured: bool,
    pub user: Option<User>,
    pub works: Vec<Work>,
    pub selected: Option<Work>,
    pub delete_pending: Option<String>,
    pub busy: bool,
    pub offset: usize,
    pub more: bool,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = blobatarCloud)]
    pub fn configured() -> bool;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn session() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud, js_name = takeDraft)]
    pub fn take_draft() -> Result<Option<String>, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn login(settings: &str) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn logout() -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn list(offset: usize) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn save(settings: &str, id: &str, revision: &str) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch, js_namespace = blobatarCloud)]
    pub fn remove(id: &str, revision: &str) -> Result<js_sys::Promise, JsValue>;
}

pub async fn resolve(promise: Result<js_sys::Promise, JsValue>) -> Result<JsValue, String> {
    wasm_bindgen_futures::JsFuture::from(promise.map_err(crate::browser::error)?)
        .await
        .map_err(crate::browser::error)
}

pub async fn json<T: serde::de::DeserializeOwned>(
    promise: Result<js_sys::Promise, JsValue>,
) -> Result<T, String> {
    let value = resolve(promise)
        .await?
        .as_string()
        .ok_or("Invalid cloud response")?;
    serde_json::from_str(&value).map_err(|error| error.to_string())
}
