use blobatar_core::Options;
use blobatar_gpui::{
    Blobatar, Drawing,
    gpui::{Context, ImageSource, Render, Window, div, img, prelude::*, px},
};
use std::sync::Arc;

/// Profile images are optional; pending or failed loads show the seeded fallback.
pub struct ProfileAvatar {
    pub label: String,
    source: Option<ImageSource>,
    fallback: Arc<Drawing>,
    size: f32,
}

impl ProfileAvatar {
    pub fn new(name: &str, source: Option<ImageSource>, options: &Options, size: f32) -> Self {
        Self {
            label: options.title.clone().unwrap_or_else(|| name.into()),
            source,
            fallback: super::drawing(name, options),
            size,
        }
    }

    pub fn set_source(&mut self, source: Option<ImageSource>, cx: &mut Context<Self>) {
        self.source = source;
        cx.notify();
    }
}

impl Render for ProfileAvatar {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let loading = self.fallback.clone();
        let fallback = self.fallback.clone();
        let content = match self.source.clone() {
            Some(source) => img(source)
                .size_full()
                .object_fit(blobatar_gpui::gpui::ObjectFit::Cover)
                .with_loading(move || {
                    Blobatar::from_drawing(loading.clone())
                        .size(size)
                        .into_any_element()
                })
                .with_fallback(move || {
                    Blobatar::from_drawing(fallback.clone())
                        .size(size)
                        .into_any_element()
                })
                .into_any_element(),
            None => Blobatar::from_drawing(self.fallback.clone())
                .size(size)
                .into_any_element(),
        };
        div()
            .size(px(size))
            .flex_shrink_0()
            .rounded_full()
            .overflow_hidden()
            .child(content)
    }
}
