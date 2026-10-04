use std::ops::Range;

use blobatar_gpui::gpui::{HighlightStyle, rgb};
use rustc_lexer::{LiteralKind, TokenKind};

const KEYWORD: u32 = 0xc792ea;
const STRING: u32 = 0xc3e88d;
const COMMENT: u32 = 0x8d9aad;
const NUMBER: u32 = 0xf78c6c;
const TYPE: u32 = 0x82aaff;
const FUNCTION: u32 = 0xffcb6b;

pub fn rust_highlights(code: &str) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut offset = 0;
    rustc_lexer::tokenize(code)
        .filter_map(|token| {
            let range = offset..offset + token.len;
            offset = range.end;
            let word = &code[range.clone()];
            let color = match token.kind {
                TokenKind::LineComment | TokenKind::BlockComment { .. } => COMMENT,
                TokenKind::Literal {
                    kind: LiteralKind::Int { .. } | LiteralKind::Float { .. },
                    ..
                } => NUMBER,
                TokenKind::Literal { .. } => STRING,
                TokenKind::Ident if matches!(word, "true" | "false") => NUMBER,
                TokenKind::Ident if is_keyword(word) => KEYWORD,
                TokenKind::Ident | TokenKind::RawIdent
                    if code[range.end..].trim_start().starts_with('(') =>
                {
                    FUNCTION
                }
                TokenKind::Ident if word.starts_with(char::is_uppercase) => TYPE,
                _ => return None,
            };
            Some((
                range,
                HighlightStyle {
                    color: Some(rgb(color).into()),
                    ..Default::default()
                },
            ))
        })
        .collect()
}

fn is_keyword(word: &str) -> bool {
    matches!(
        word,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(code: &str) -> Vec<(&str, u32)> {
        rust_highlights(code)
            .into_iter()
            .map(|(range, style)| {
                let color = [KEYWORD, STRING, COMMENT, NUMBER, TYPE, FUNCTION]
                    .into_iter()
                    .find(|color| style.color == Some(rgb(*color).into()))
                    .unwrap();
                (&code[range], color)
            })
            .collect()
    }

    #[test]
    fn rust_tokens_keep_strings_and_nested_comments_intact() {
        let code = "let name = \"日🦀\\\"let\"; /* fn /* inner */ true */\n// use\nOptions::new(192.0, false);";
        assert_eq!(
            tokens(code),
            vec![
                ("let", KEYWORD),
                ("\"日🦀\\\"let\"", STRING),
                ("/* fn /* inner */ true */", COMMENT),
                ("// use", COMMENT),
                ("Options", TYPE),
                ("new", FUNCTION),
                ("192.0", NUMBER),
                ("false", NUMBER),
            ]
        );
        assert!(rust_highlights("").is_empty());
    }

    #[test]
    fn generated_code_highlights_preserve_utf8_and_raw_json_boundaries() {
        let mut state = crate::editor::EditorState::default();
        state.settings.name = "日🦀\"\\".into();
        state.settings.options.title = Some("literal \"# marker".into());
        let code = crate::snippet::snippet(&state);
        let highlights = rust_highlights(&code);
        let mut reconstructed = String::new();
        let mut end = 0;
        for (range, _) in highlights {
            assert!(range.start >= end);
            reconstructed.push_str(&code[end..range.start]);
            reconstructed.push_str(&code[range.clone()]);
            end = range.end;
        }
        reconstructed.push_str(&code[end..]);
        assert_eq!(reconstructed, code);
        let tokens = tokens(&code);
        assert!(tokens.iter().any(|(text, color)| *color == STRING
            && text.starts_with("r##\"")
            && text.ends_with("\"##")));
        assert!(tokens.contains(&("with_generation", FUNCTION)));
        assert!(tokens.contains(&("set_reduced_motion", FUNCTION)));
    }
}
