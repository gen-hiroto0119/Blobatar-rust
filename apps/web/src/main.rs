#[cfg(target_family = "wasm")]
fn main() {
    use blobatar_gpui::gpui::{App, AppContext, Focusable, WindowOptions};
    gpui_platform::web_init();
    gpui_platform::single_threaded_web().run(|cx: &mut App| {
        gpui_base::init(cx);
        blobatar_ui::init(cx);
        cx.open_window(WindowOptions::default(), |window, cx| {
            let editor = cx.new(blobatar_ui::Editor::new);
            editor.read(cx).focus_handle(cx).focus(window, cx);
            cx.new(|cx| gpui_base::Root::new(editor, window, cx))
        })
        .expect("open editor");
        cx.activate(true);
    });
}

#[cfg(not(target_family = "wasm"))]
fn main() {
    eprintln!("Run `trunk serve` from apps/web to open the browser editor.");
}
