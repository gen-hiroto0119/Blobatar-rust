mod advanced_json;
pub mod axes;
mod code_highlight;
pub mod components;
pub mod editor;
pub mod native;
pub mod snippet;
mod text_input;
pub mod wall;
pub mod wall_camera;

pub use native::Editor;
pub use wall::Wall;

pub fn init(cx: &mut blobatar_gpui::gpui::App) {
    text_input::init(cx);
}
