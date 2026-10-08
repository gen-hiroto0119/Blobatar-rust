mod advanced_json;
pub mod axes;
#[cfg(target_family = "wasm")]
mod browser;
#[cfg(target_family = "wasm")]
mod cloud;
mod code_highlight;
#[cfg(not(target_family = "wasm"))]
pub mod components;
pub mod editor;
pub mod native;
pub mod snippet;
mod text_input;
pub mod theme;
#[cfg(not(target_family = "wasm"))]
pub mod wall;
#[cfg(not(target_family = "wasm"))]
pub mod wall_camera;

pub use native::Editor;
pub use text_input::edit_menu;
#[cfg(not(target_family = "wasm"))]
pub use wall::Wall;

pub fn init(cx: &mut blobatar_gpui::gpui::App) {
    theme::init(cx);
    text_input::init(cx);
}
