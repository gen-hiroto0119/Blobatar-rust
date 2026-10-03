use std::collections::HashSet;

use blobatar_core::{Background, Expression, Generation, Options, hash::is_ecmascript_whitespace};
use url::form_urlencoded;

use crate::error::ApiError;

pub(crate) const QUERY_PARAMETER_NAMES: &[&str] = &[
    "s",
    "size",
    "background",
    "hue",
    "tone",
    "expression",
    "title",
    "gen",
    "d",
    "default",
    "f",
    "forcedefault",
    "r",
    "rating",
];
pub(crate) const BACKGROUND_NAMES: &[&str] = &["none", "square", "circle", "squircle"];
pub(crate) const SIZE_MIN: f64 = 8.0;
pub(crate) const SIZE_MAX: f64 = 1024.0;
pub(crate) const HUE_MIN: f64 = 0.0;
pub(crate) const HUE_MAX: f64 = 360.0;
pub(crate) const TONE_MIN: f64 = 0.0;
pub(crate) const TONE_MAX: f64 = 1.0;
pub(crate) const NAME_MAX_UTF16: usize = 256;
pub(crate) const TITLE_MAX_UTF16: usize = 128;

#[derive(Debug)]
pub(crate) struct ParsedOptions {
    pub(crate) options: Options,
    pub(crate) generation: Generation,
    pub(crate) generation_pinned: bool,
}

pub(crate) fn parse_options(query: Option<&str>) -> Result<ParsedOptions, ApiError> {
    let mut first = Vec::new();
    let mut seen = HashSet::new();
    if let Some(query) = query {
        for (key, value) in form_urlencoded::parse(query.as_bytes()) {
            let key = key.into_owned();
            if !QUERY_PARAMETER_NAMES.contains(&key.as_str()) {
                return Err(unknown_parameter(&key));
            }
            if seen.insert(key.clone()) {
                first.push((key, value.into_owned()));
            }
        }
    }

    let mut options = Options::default();
    let mut generation = Generation::Two;
    let mut generation_pinned = false;
    for (key, value) in &first {
        match key.as_str() {
            "background" => {
                if !BACKGROUND_NAMES.contains(&value.as_str()) {
                    return Err(unknown_value("background", value, BACKGROUND_NAMES));
                }
                options.background = match value.as_str() {
                    "none" => Some(Background::Enabled(false)),
                    kind => Some(Background::Kind(kind.to_owned())),
                };
            }
            "hue" => {
                options.hue = Some(parse_in_range("hue", value, HUE_MIN, HUE_MAX)?);
            }
            "tone" => {
                options.tone = Some(parse_in_range("tone", value, TONE_MIN, TONE_MAX)?);
            }
            "expression" => {
                let expression = Expression::ALL
                    .into_iter()
                    .find(|expression| expression.name() == value)
                    .ok_or_else(|| unknown_value("expression", value, &expression_names()))?;
                options.expression = Some(expression);
            }
            "title" => {
                let length = value.encode_utf16().count();
                if length > TITLE_MAX_UTF16 {
                    return Err(ApiError::bad_request(
                        "title_too_long",
                        format!(
                            "title must be {TITLE_MAX_UTF16} characters or fewer, got {length}"
                        ),
                    ));
                }
                options.title = Some(value.clone());
            }
            "gen" => {
                generation_pinned = true;
                generation = match value.as_str() {
                    "1" => Generation::One,
                    "2" => Generation::Two,
                    _ => return Err(unknown_value("gen", value, &["1", "2"])),
                };
            }
            _ => {}
        }
    }

    let size_value = first
        .iter()
        .find(|(key, _)| key == "s")
        .or_else(|| first.iter().find(|(key, _)| key == "size"))
        .map(|(_, value)| value);
    if let Some(value) = size_value
        && let Some(number) = parse_ecmascript_number(value)
    {
        let clamped = number.clamp(SIZE_MIN, SIZE_MAX);
        options.size = Some((clamped + 0.5).floor());
    }

    Ok(ParsedOptions {
        options,
        generation,
        generation_pinned,
    })
}

pub(crate) fn parse_name(path_name: &str) -> Result<String, ApiError> {
    if path_name.contains('/') {
        return Err(ApiError::bad_request(
            "name_has_slash",
            "expected /avatar/<name> — a name containing a slash must be percent-encoded as %2F",
        ));
    }

    let name = strip_literal_extension(path_name);
    let name = percent_decode(name).ok_or_else(|| {
        ApiError::bad_request("name_encoding", "name is not valid percent-encoding")
    })?;
    if name.is_empty() {
        return Err(ApiError::bad_request("name_empty", "name is empty"));
    }

    let length = name.encode_utf16().count();
    if length > NAME_MAX_UTF16 {
        return Err(ApiError::bad_request(
            "name_too_long",
            format!("name must be {NAME_MAX_UTF16} characters or fewer, got {length}"),
        ));
    }
    Ok(name)
}

fn strip_literal_extension(name: &str) -> &str {
    [".svg", ".png", ".jpg", ".jpeg", ".gif", ".webp"]
        .into_iter()
        .find_map(|extension| {
            let start = name.len().checked_sub(extension.len())?;
            name.get(start..)
                .filter(|suffix| suffix.eq_ignore_ascii_case(extension))
                .map(|_| &name[..start])
        })
        .unwrap_or(name)
}

fn percent_decode(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let input = value.as_bytes();
    let mut index = 0;
    while index < input.len() {
        if input[index] == b'%' {
            let high = *input.get(index + 1)?;
            let low = *input.get(index + 2)?;
            bytes.push(hex_value(high)? << 4 | hex_value(low)?);
            index += 3;
        } else {
            bytes.push(input[index]);
            index += 1;
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn parse_in_range(name: &str, value: &str, min: f64, max: f64) -> Result<f64, ApiError> {
    let number = parse_ecmascript_number(value).ok_or_else(|| {
        ApiError::bad_request(
            "invalid_number",
            format!("{name} must be a number, got {}", json_string(value)),
        )
    })?;
    if number < min || number > max {
        let mut buffer = ryu_js::Buffer::new();
        return Err(ApiError::bad_request(
            "out_of_range",
            format!(
                "{name} must be between {} and {}, got {}",
                display_number(min),
                display_number(max),
                buffer.format_finite(number)
            ),
        ));
    }
    Ok(number)
}

fn parse_ecmascript_number(value: &str) -> Option<f64> {
    let value = value.trim_matches(is_ecmascript_whitespace);
    if value.is_empty() {
        return None;
    }
    let number = if let Some(digits) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        parse_radix_number(digits, 16)?
    } else if let Some(digits) = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
    {
        parse_radix_number(digits, 8)?
    } else if let Some(digits) = value
        .strip_prefix("0b")
        .or_else(|| value.strip_prefix("0B"))
    {
        parse_radix_number(digits, 2)?
    } else {
        value.parse::<f64>().ok()?
    };
    number.is_finite().then_some(number)
}

fn parse_radix_number(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }
    let digits: Vec<u32> = digits
        .chars()
        .map(|digit| digit.to_digit(radix))
        .collect::<Option<_>>()?;
    let value = u128::from_str_radix(
        &digits
            .iter()
            .map(|digit| char::from_digit(*digit, radix).unwrap())
            .collect::<String>(),
        radix,
    )
    .map(|number| number as f64)
    .unwrap_or_else(|_| {
        digits.iter().fold(0.0, |number, digit| {
            number * f64::from(radix) + f64::from(*digit)
        })
    });
    value.is_finite().then_some(value)
}

fn unknown_parameter(name: &str) -> ApiError {
    ApiError::bad_request(
        "unknown_parameter",
        format!(
            "unknown parameter {} — expected one of {}",
            json_string(name),
            QUERY_PARAMETER_NAMES.join(", ")
        ),
    )
}

fn unknown_value(name: &str, value: &str, expected: &[&str]) -> ApiError {
    ApiError::bad_request(
        "unknown_value",
        format!(
            "unknown {name} {} — expected one of {}",
            json_string(value),
            expected.join(", ")
        ),
    )
}

fn expression_names() -> Vec<&'static str> {
    Expression::ALL.into_iter().map(Expression::name).collect()
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize as JSON")
}

fn display_number(value: f64) -> String {
    let mut buffer = ryu_js::Buffer::new();
    buffer.format_finite(value).to_owned()
}
