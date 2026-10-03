pub mod axes;
pub mod components;
pub mod editor;
pub mod native;
pub mod snippet;
mod text_input;

pub use native::Editor;

pub fn init(cx: &mut blobatar_gpui::gpui::App) {
    text_input::init(cx);
}
