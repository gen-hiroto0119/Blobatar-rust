use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

pub const MAX_NAME: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRefusal {
    Empty,
    Long,
    Charset,
    Blocked,
}

pub fn check_name(raw: Option<&str>, extra_blocklist: Option<&str>) -> Result<String, NameRefusal> {
    let raw = raw.ok_or(NameRefusal::Empty)?;
    let name = raw.trim_matches(is_ecmascript_whitespace);
    if name.is_empty() {
        return Err(NameRefusal::Empty);
    }
    if name.chars().count() > MAX_NAME {
        return Err(NameRefusal::Long);
    }
    if !valid_shape(name) {
        return Err(NameRefusal::Charset);
    }
    let folded = fold_name(name);
    if blocklist(extra_blocklist)
        .iter()
        .any(|term| folded.contains(term))
    {
        return Err(NameRefusal::Blocked);
    }
    Ok(name.to_owned())
}

pub fn check_expression(raw: Option<&str>) -> bool {
    let Some(expression) = raw else {
        return false;
    };
    (2..=16).contains(&expression.len()) && expression.bytes().all(|byte| byte.is_ascii_lowercase())
}

pub fn fold_name(name: &str) -> String {
    let mut folded = String::new();
    for character in name.nfkd() {
        if character.general_category_group() == GeneralCategoryGroup::Mark {
            continue;
        }
        for lower in character.to_lowercase() {
            let mapped = match lower {
                '0' | '@' => 'o',
                '1' | '!' | '|' => 'i',
                '3' => 'e',
                '4' => 'a',
                '5' | '$' => 's',
                '7' => 't',
                character => character,
            };
            if mapped.general_category_group() != GeneralCategoryGroup::Letter {
                continue;
            }
            if folded.ends_with(mapped) {
                continue;
            }
            folded.push(mapped);
        }
    }
    folded
}

fn valid_shape(name: &str) -> bool {
    let characters: Vec<_> = name.chars().collect();
    let is_word = |character: char| {
        matches!(
            character.general_category_group(),
            GeneralCategoryGroup::Letter
                | GeneralCategoryGroup::Mark
                | GeneralCategoryGroup::Number
        )
    };
    if !characters
        .first()
        .is_some_and(|character| is_word(*character))
        || !characters
            .last()
            .is_some_and(|character| is_word(*character))
    {
        return false;
    }
    if characters
        .iter()
        .any(|character| !is_word(*character) && !matches!(*character, ' ' | '\'' | '-' | '.'))
    {
        return false;
    }
    !characters
        .windows(2)
        .any(|pair| pair[0] == ' ' && pair[1] == ' ')
        && !characters.windows(3).any(|pair| {
            pair.iter()
                .all(|character| matches!(character, ' ' | '\'' | '-' | '.'))
        })
}

fn blocklist(extra: Option<&str>) -> Vec<String> {
    [
        "nigger", "faggot", "retard", "tranny", "kike", "chink", "rapist", "hitler",
    ]
    .into_iter()
    .chain(extra.unwrap_or_default().split(','))
    .map(fold_name)
    .filter(|term| term.chars().count() > 2)
    .collect()
}

fn is_ecmascript_whitespace(character: char) -> bool {
    matches!(
        character,
        '\u{0009}'..='\u{000d}'
            | '\u{0020}'
            | '\u{00a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}
