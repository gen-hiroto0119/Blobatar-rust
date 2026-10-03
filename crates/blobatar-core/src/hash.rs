use unicode_normalization::UnicodeNormalization;

pub fn is_ecmascript_whitespace(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn normalize_seed(seed: &str) -> String {
    seed.nfc()
        .collect::<String>()
        .trim_matches(is_ecmascript_whitespace)
        .to_lowercase()
}

fn feed(mut h: u32, bytes: impl IntoIterator<Item = u8>) -> u32 {
    for byte in bytes {
        h = (h ^ u32::from(byte))
            .wrapping_mul(3_432_918_353)
            .rotate_left(13);
    }
    h
}

pub fn seed_state(seed: &str, normalize: bool) -> u32 {
    let normalized;
    let s = if normalize {
        normalized = normalize_seed(seed);
        &normalized
    } else {
        seed
    };
    feed(1_779_033_703 ^ s.encode_utf16().count() as u32, s.bytes())
}

pub fn stream(state: u32, key: &str) -> f64 {
    let mut h = feed(feed(state, [0xff]), key.bytes());
    h = (h ^ (h >> 16)).wrapping_mul(2_246_822_507);
    h = (h ^ (h >> 13)).wrapping_mul(3_266_489_909);
    f64::from(h ^ (h >> 16)) / 4_294_967_296.0
}
