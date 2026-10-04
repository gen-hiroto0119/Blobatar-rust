use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityHash(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenHash(String);

impl IdentityHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TokenHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn hash_identity(identity: &str, day: &str, secret: &str) -> IdentityHash {
    IdentityHash(hex_digest(format!("{secret}:{day}:{identity}").as_bytes()))
}

pub fn hash_token(token: &str) -> TokenHash {
    TokenHash(hex_digest(token.as_bytes()))
}

pub fn new_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn is_token(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn token_from_cookie(cookie: &str) -> Option<&str> {
    for part in cookie.split(';') {
        let Some((name, value)) = part.trim().split_once('=') else {
            continue;
        };
        if name == "wall" {
            return is_token(value).then_some(value);
        }
    }
    None
}

pub fn day_of(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

pub fn next_midnight(seconds: i64) -> i64 {
    seconds.div_euclid(86_400) * 86_400 + 86_400
}

pub fn same_secret(offered: &str, expected: &str) -> bool {
    offered.as_bytes().ct_eq(expected.as_bytes()).into()
}

fn hex_digest(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn civil_from_days(days: i64) -> (i64, u8, u8) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u8, day as u8)
}
