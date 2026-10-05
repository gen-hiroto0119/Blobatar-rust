//! Plexer-derived presentation tokens. Avatar generation and saved settings are independent.
use std::borrow::Cow;

use blobatar_gpui::gpui::{
    App, Div, Font, FontFallbacks, FontWeight, Hsla, SharedString, Stateful, div, font, prelude::*,
    px, rgb, rgba,
};

pub const BODY_FONT: &str = "Noto Sans JP";
pub const HEADING_FONT: &str = "Inter";
pub const CODE_FONT: &str = "IBM Plex Mono";
pub const CONTROL_RADIUS: f32 = 8.0;
pub const PANEL_RADIUS: f32 = 12.0;
pub const BUTTON_HEIGHT: f32 = 36.0;
pub const INPUT_HEIGHT: f32 = 40.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    #[default]
    Light,
    Dark,
}

impl Appearance {
    pub fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    pub fn palette(self) -> Palette {
        let dark = self == Self::Dark;
        let surface: Hsla = rgb(if dark { 0x1d1c1a } else { 0xffffff }).into();
        Palette {
            surface,
            background: if dark {
                surface.blend(rgba(0xf2efea0a).into())
            } else {
                rgb(0xf0f0ee).into()
            },
            text: rgb(if dark { 0xf2efea } else { 0x1d1c1a }).into(),
            muted: rgb(if dark { 0xaaa49e } else { 0x77736f }).into(),
            border: if dark {
                rgba(0xf2efea1f).into()
            } else {
                rgb(0xdddcda).into()
            },
            hover: surface.blend(if dark {
                rgba(0xf2efea14).into()
            } else {
                rgba(0x1d1c1a0f).into()
            }),
            action: rgb(if dark { 0xffffff } else { 0x1d1c1a }).into(),
            action_text: rgb(if dark { 0x1d1c1a } else { 0xffffff }).into(),
            accent: rgb(0xef551a).into(),
            selected_text: rgb(if dark { 0xef551a } else { 0xb74416 }).into(),
            selected: surface.blend(rgba(0xef551a14).into()),
            selection: rgba(0xef551a40).into(),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Hsla,
    pub surface: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub border: Hsla,
    pub hover: Hsla,
    pub action: Hsla,
    pub action_text: Hsla,
    pub accent: Hsla,
    pub selected_text: Hsla,
    pub selected: Hsla,
    pub selection: Hsla,
}

#[derive(Clone, Copy)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Selected,
}

#[derive(Clone, Copy)]
pub struct ButtonStyle {
    pub kind: ButtonKind,
    pub disabled: bool,
}

impl ButtonStyle {
    pub fn primary(disabled: bool) -> Self {
        Self {
            kind: ButtonKind::Primary,
            disabled,
        }
    }

    pub fn secondary(disabled: bool) -> Self {
        Self {
            kind: ButtonKind::Secondary,
            disabled,
        }
    }
}

impl From<bool> for ButtonStyle {
    fn from(selected: bool) -> Self {
        Self {
            kind: if selected {
                ButtonKind::Selected
            } else {
                ButtonKind::Secondary
            },
            disabled: false,
        }
    }
}

pub fn button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    appearance: Appearance,
    style: ButtonStyle,
) -> Stateful<Div> {
    let palette = appearance.palette();
    let (background, text, border) = match style.kind {
        ButtonKind::Primary => (palette.action, palette.action_text, palette.action),
        ButtonKind::Secondary => (palette.surface, palette.text, palette.border),
        ButtonKind::Selected => (palette.selected, palette.selected_text, palette.accent),
    };
    div()
        .id(id.into())
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .h(px(BUTTON_HEIGHT))
        .px_4()
        .rounded(px(CONTROL_RADIUS))
        .border_1()
        .border_color(border)
        .bg(background)
        .text_color(text)
        .font_family(BODY_FONT)
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(13.0))
        .line_height(px(20.0))
        .whitespace_nowrap()
        .when(style.disabled, |button| button.opacity(0.45))
        .when(!style.disabled, |button| {
            button
                .focusable()
                .tab_stop(true)
                .cursor_pointer()
                .hover(move |hover| {
                    hover.bg(match style.kind {
                        ButtonKind::Primary => background.blend(text.opacity(0.10)),
                        _ => palette.hover,
                    })
                })
                .focus(move |focus| focus.border_2().border_color(palette.accent))
        })
        .child(label.into())
}

pub fn panel(appearance: Appearance) -> Div {
    let palette = appearance.palette();
    div()
        .flex()
        .flex_col()
        .gap_4()
        .p_4()
        .flex_shrink_0()
        .rounded(px(PANEL_RADIUS))
        .border_1()
        .border_color(palette.border)
        .bg(palette.surface)
}

pub fn heading(label: impl Into<SharedString>) -> Div {
    div()
        .font(font_with_japanese_fallback(HEADING_FONT))
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(18.0))
        .line_height(px(28.0))
        .child(label.into())
}

pub fn font_with_japanese_fallback(family: &'static str) -> Font {
    Font {
        fallbacks: Some(FontFallbacks::from_fonts(vec![BODY_FONT.into()])),
        ..font(family)
    }
}

pub(crate) fn init(cx: &mut App) {
    cx.text_system()
        .add_fonts(vec![
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/NotoSansJP.ttf")),
            Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf")),
        ])
        .expect("bundled OFL fonts must load");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn themes_preserve_semantic_colors_and_transparency() {
        let light = Appearance::Light.palette();
        let dark = Appearance::Dark.palette();
        assert_eq!(light.action, rgb(0x1d1c1a).into());
        assert_eq!(dark.action, rgb(0xffffff).into());
        assert_eq!(light.accent, dark.accent);
        assert_eq!(dark.background, dark.surface.blend(rgba(0xf2efea0a).into()));
        assert_eq!(dark.border, rgba(0xf2efea1f).into());
        assert_eq!(Appearance::default().toggled().toggled(), Appearance::Light);
    }
}
