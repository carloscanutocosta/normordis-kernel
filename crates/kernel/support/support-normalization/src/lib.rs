use chrono::NaiveDate;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use thiserror::Error;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NormalizationError {
    #[error("data invalida: {0}")]
    InvalidDate(String),
    #[error("valor monetario invalido")]
    InvalidMoney,
    #[error("numero invalido")]
    InvalidNumber,
    #[error("numero fora do intervalo suportado")]
    NumberOutOfRange,
}

pub fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn trim_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn normalize_unicode_nfc(value: &str) -> String {
    value.nfc().collect()
}

pub fn normalize_unicode_nfd(value: &str) -> String {
    value.nfd().collect()
}

pub fn normalize_unicode_nfkc(value: &str) -> String {
    value.nfkc().collect()
}

pub fn normalize_unicode_nfkd(value: &str) -> String {
    value.nfkd().collect()
}

pub fn strip_diacritics(value: &str) -> String {
    value.nfd().filter(|ch| !is_combining_mark(*ch)).collect()
}

pub fn normalize_for_lookup(value: &str) -> String {
    normalize_whitespace(&strip_diacritics(value))
        .to_lowercase()
        .trim()
        .to_string()
}

pub fn capitalize_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => {
            let mut result = first.to_uppercase().collect::<String>();
            result.push_str(chars.as_str());
            result
        }
        None => String::new(),
    }
}

pub fn title_case(value: &str) -> String {
    normalize_whitespace(value)
        .split(' ')
        .map(|word| {
            let lower = word.to_lowercase();
            capitalize_first(&lower)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn normalize_portuguese_name(value: &str) -> String {
    let particles = ["e", "de", "da", "do", "das", "dos", "a", "o", "as", "os"];

    normalize_whitespace(value)
        .split(' ')
        .enumerate()
        .map(|(index, word)| {
            let lower = word.to_lowercase();
            if index > 0 && particles.contains(&lower.as_str()) {
                lower
            } else {
                capitalize_compound_name_part(&lower)
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn digits_only(value: &str) -> String {
    value.chars().filter(|ch| ch.is_ascii_digit()).collect()
}

pub fn letters_and_digits_only(value: &str) -> String {
    value.chars().filter(|ch| ch.is_alphanumeric()).collect()
}

pub fn parse_i64_loose(value: &str) -> Option<i64> {
    let cleaned = normalize_integer_string(value)?;
    cleaned.parse::<i64>().ok()
}

pub fn parse_f64_loose(value: &str) -> Option<f64> {
    let cleaned = normalize_decimal_string(value)?;
    cleaned.parse::<f64>().ok()
}

pub fn round_to_places(value: f64, places: u32) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let factor = 10_f64.powi(places as i32);
    (value * factor).round() / factor
}

pub fn money_to_cents(value: f64) -> Result<i64, NormalizationError> {
    if !value.is_finite() {
        return Err(NormalizationError::InvalidMoney);
    }
    let decimal = Decimal::from_f64(value).ok_or(NormalizationError::InvalidMoney)?;
    money_decimal_to_cents(decimal)
}

pub fn money_decimal_to_cents(value: Decimal) -> Result<i64, NormalizationError> {
    if value.scale() > 2 {
        return Err(NormalizationError::InvalidMoney);
    }
    let cents = value
        .checked_mul(Decimal::new(100, 0))
        .ok_or(NormalizationError::NumberOutOfRange)?;
    cents.to_i64().ok_or(NormalizationError::NumberOutOfRange)
}

pub fn money_str_to_cents(value: &str) -> Result<i64, NormalizationError> {
    let (negative, integer, fraction) =
        parse_decimal_parts(value, Some(2)).ok_or(NormalizationError::InvalidMoney)?;
    if fraction.is_empty() && value.chars().any(|ch| matches!(ch, '.' | ',')) {
        return Err(NormalizationError::InvalidMoney);
    }
    let euros = parse_u64_digits(&integer).ok_or(NormalizationError::InvalidMoney)?;
    if euros > 999_999_999_999 {
        return Err(NormalizationError::NumberOutOfRange);
    }

    let cents = match fraction.len() {
        0 => 0,
        1 => parse_u64_digits(&fraction).ok_or(NormalizationError::InvalidMoney)? * 10,
        2 => parse_u64_digits(&fraction).ok_or(NormalizationError::InvalidMoney)?,
        _ => return Err(NormalizationError::InvalidMoney),
    };
    let total = euros
        .checked_mul(100)
        .and_then(|v| v.checked_add(cents))
        .ok_or(NormalizationError::NumberOutOfRange)?;
    let signed = i64::try_from(total).map_err(|_| NormalizationError::NumberOutOfRange)?;
    Ok(if negative { -signed } else { signed })
}

pub fn number_to_words_pt(value: i64) -> Result<String, NormalizationError> {
    if value == 0 {
        return Ok("zero".to_string());
    }
    if value.unsigned_abs() > 999_999_999_999_u64 {
        return Err(NormalizationError::NumberOutOfRange);
    }
    if value < 0 {
        return Ok(format!(
            "menos {}",
            number_to_words_positive((-value) as u64)
        ));
    }
    Ok(number_to_words_positive(value as u64))
}

pub fn money_to_words_eur(value: f64) -> Result<String, NormalizationError> {
    let cents = money_to_cents(value)?;
    money_cents_to_words_eur(cents)
}

pub fn money_str_to_words_eur(value: &str) -> Result<String, NormalizationError> {
    let cents = money_str_to_cents(value)?;
    money_cents_to_words_eur(cents)
}

pub fn money_cents_to_words_eur(cents: i64) -> Result<String, NormalizationError> {
    let abs = cents.unsigned_abs();
    let euros = abs / 100;
    let centimos = abs % 100;
    if euros > 999_999_999_999 {
        return Err(NormalizationError::NumberOutOfRange);
    }

    let negative = cents < 0;

    if euros == 0 && centimos > 0 {
        let centimos_words = if centimos == 1 {
            "um cêntimo".to_string()
        } else {
            format!("{} cêntimos", number_to_words_positive(centimos))
        };
        return Ok(if negative {
            format!("menos {centimos_words}")
        } else {
            centimos_words
        });
    }

    let euros_words = if euros == 1 {
        "um euro".to_string()
    } else {
        format!("{} euros", number_to_words_positive(euros))
    };

    let mut result = if negative {
        format!("menos {euros_words}")
    } else {
        euros_words
    };

    if centimos > 0 {
        let centimos_words = if centimos == 1 {
            "um cêntimo".to_string()
        } else {
            format!("{} cêntimos", number_to_words_positive(centimos))
        };
        result.push_str(" e ");
        result.push_str(&centimos_words);
    }

    Ok(result)
}

pub fn normalize_date_to_iso(value: &str) -> Result<String, NormalizationError> {
    let value = normalize_whitespace(value);
    let value = value.trim();
    for format in ["%Y-%m-%d", "%Y/%m/%d", "%d-%m-%Y", "%d/%m/%Y"] {
        if let Ok(date) = NaiveDate::parse_from_str(value, format) {
            return Ok(date.format("%Y-%m-%d").to_string());
        }
    }
    Err(NormalizationError::InvalidDate(value.to_string()))
}

pub fn is_valid_nif(value: &str) -> bool {
    let digits = digits_only(value);
    if digits.len() != 9 {
        return false;
    }
    let bytes = digits.as_bytes();
    if !matches!(
        bytes[0],
        b'1' | b'2' | b'3' | b'5' | b'6' | b'7' | b'8' | b'9'
    ) {
        return false;
    }

    let mut sum = 0_u32;
    for (index, byte) in bytes.iter().take(8).enumerate() {
        let digit = (byte - b'0') as u32;
        sum += digit * (9 - index as u32);
    }
    let check = 11 - (sum % 11);
    let expected = if check >= 10 { 0 } else { check };
    expected == (bytes[8] - b'0') as u32
}

pub fn is_valid_email(value: &str) -> bool {
    let value = value.trim();
    let (local, domain) = match value.split_once('@') {
        Some(parts) => parts,
        None => return false,
    };
    let domain_ascii = match normalize_domain_to_ascii(domain) {
        Some(domain) => domain,
        None => return false,
    };
    if local.is_empty()
        || domain_ascii.is_empty()
        || value.matches('@').count() != 1
        || local.len() > 64
        || local.len() + 1 + domain_ascii.len() > 254
    {
        return false;
    }
    if domain_ascii.starts_with('.')
        || domain_ascii.ends_with('.')
        || domain_ascii.contains("..")
        || !domain_ascii.contains('.')
    {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    let local_valid = local
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-'));
    let domain_valid = domain_ascii.split('.').all(is_valid_domain_label);
    let tld_valid = domain_ascii
        .rsplit('.')
        .next()
        .is_some_and(is_valid_tld_label);
    local_valid && domain_valid && tld_valid
}

pub fn normalize_domain_to_ascii(domain: &str) -> Option<String> {
    let domain = normalize_unicode_nfkc(domain.trim());
    if domain.is_empty() || domain.chars().any(char::is_whitespace) {
        return None;
    }
    idna::domain_to_ascii(&domain)
        .ok()
        .map(|ascii| ascii.to_ascii_lowercase())
}

fn normalize_integer_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, body) = split_sign(trimmed);
    if body.is_empty()
        || body
            .chars()
            .any(|ch| !ch.is_ascii_digit() && !is_group_separator(ch))
    {
        return None;
    }

    let digits = body
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() || !valid_integer_grouping(body) {
        None
    } else {
        Some(format!("{sign}{digits}"))
    }
}

fn normalize_decimal_string(value: &str) -> Option<String> {
    let (negative, integer, fraction) = parse_decimal_parts(value, None)?;
    let sign = if negative { "-" } else { "" };
    if fraction.is_empty() {
        Some(format!("{sign}{integer}"))
    } else {
        Some(format!("{sign}{integer}.{fraction}"))
    }
}

fn parse_decimal_parts(value: &str, max_fraction: Option<usize>) -> Option<(bool, String, String)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (sign, body) = split_sign(trimmed);
    let negative = sign == "-";
    if body.is_empty()
        || body
            .chars()
            .any(|ch| !ch.is_ascii_digit() && !matches!(ch, '.' | ',' | ' '))
    {
        return None;
    }

    let decimal_index = find_decimal_separator(body);
    let (integer_part, fraction_part) = match decimal_index {
        Some(index) => (&body[..index], &body[index + 1..]),
        None => (body, ""),
    };

    if integer_part.is_empty()
        || !valid_integer_grouping(integer_part)
        || fraction_part.chars().any(|ch| !ch.is_ascii_digit())
        || fraction_part.contains(' ')
        || (fraction_part.is_empty() && decimal_index.is_some())
    {
        return None;
    }
    if max_fraction.is_some_and(|max| fraction_part.len() > max) {
        return None;
    }

    let integer = integer_part
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let fraction = fraction_part.to_string();
    if integer.is_empty() {
        None
    } else {
        Some((negative, integer, fraction))
    }
}

fn find_decimal_separator(value: &str) -> Option<usize> {
    let last_dot = value.rfind('.');
    let last_comma = value.rfind(',');
    match (last_dot, last_comma) {
        (Some(dot), Some(comma)) => Some(dot.max(comma)),
        (Some(index), None) | (None, Some(index)) => {
            let sep = value.as_bytes()[index] as char;
            let count = value.chars().filter(|&ch| ch == sep).count();
            let right_len = value[index + 1..].chars().count();
            if count == 1 && right_len != 3 {
                Some(index)
            } else {
                None
            }
        }
        (None, None) => None,
    }
}

fn valid_integer_grouping(value: &str) -> bool {
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    let separators = value
        .chars()
        .filter(|&ch| is_group_separator(ch))
        .collect::<std::collections::HashSet<_>>();
    if separators.len() != 1 {
        return false;
    }

    let Some(&separator) = separators.iter().next() else {
        return false;
    };
    let groups = value.split(separator).collect::<Vec<_>>();
    if groups.len() < 2
        || groups[0].is_empty()
        || groups[0].len() > 3
        || !groups[0].chars().all(|ch| ch.is_ascii_digit())
    {
        return false;
    }
    groups[1..]
        .iter()
        .all(|group| group.len() == 3 && group.chars().all(|ch| ch.is_ascii_digit()))
}

fn split_sign(value: &str) -> (&str, &str) {
    if let Some(rest) = value.strip_prefix('+') {
        ("", rest)
    } else if let Some(rest) = value.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", value)
    }
}

fn is_group_separator(ch: char) -> bool {
    matches!(ch, '.' | ',' | ' ')
}

fn parse_u64_digits(value: &str) -> Option<u64> {
    if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    value.parse::<u64>().ok()
}

fn is_valid_domain_label(label: &str) -> bool {
    !label.is_empty()
        && label.len() <= 63
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn is_valid_tld_label(label: &str) -> bool {
    if label.starts_with("xn--") {
        return is_valid_domain_label(label) && label.len() > 4;
    }
    label.len() >= 2 && label.chars().all(|ch| ch.is_ascii_alphabetic())
}

fn capitalize_compound_name_part(value: &str) -> String {
    value
        .split('-')
        .map(capitalize_first)
        .collect::<Vec<_>>()
        .join("-")
}

fn number_to_words_positive(value: u64) -> String {
    if value < 1000 {
        return under_thousand_to_words(value as u16);
    }

    let scales = [
        (1_000_000_000_u64, "mil milhões", "mil milhões"),
        (1_000_000_u64, "milhão", "milhões"),
        (1_000_u64, "mil", "mil"),
    ];

    for (divisor, singular, plural) in scales {
        if value >= divisor {
            let major = value / divisor;
            let remainder = value % divisor;

            let major_words = if divisor == 1_000 && major == 1 {
                "mil".to_string()
            } else if major == 1 {
                format!("um {singular}")
            } else {
                format!("{} {}", number_to_words_positive(major), plural)
            };

            if remainder == 0 {
                return major_words;
            }

            let connector = if remainder < 100 { " e " } else { ", " };
            return format!(
                "{major_words}{connector}{}",
                number_to_words_positive(remainder)
            );
        }
    }

    String::new()
}

fn under_thousand_to_words(value: u16) -> String {
    const UNITS: [&str; 20] = [
        "zero",
        "um",
        "dois",
        "três",
        "quatro",
        "cinco",
        "seis",
        "sete",
        "oito",
        "nove",
        "dez",
        "onze",
        "doze",
        "treze",
        "catorze",
        "quinze",
        "dezasseis",
        "dezassete",
        "dezoito",
        "dezanove",
    ];
    const TENS: [&str; 10] = [
        "",
        "",
        "vinte",
        "trinta",
        "quarenta",
        "cinquenta",
        "sessenta",
        "setenta",
        "oitenta",
        "noventa",
    ];
    const HUNDREDS: [&str; 10] = [
        "",
        "cento",
        "duzentos",
        "trezentos",
        "quatrocentos",
        "quinhentos",
        "seiscentos",
        "setecentos",
        "oitocentos",
        "novecentos",
    ];

    if value < 20 {
        return UNITS[value as usize].to_string();
    }
    if value < 100 {
        let tens = value / 10;
        let units = value % 10;
        if units == 0 {
            return TENS[tens as usize].to_string();
        }
        return format!("{} e {}", TENS[tens as usize], UNITS[units as usize]);
    }
    if value == 100 {
        return "cem".to_string();
    }

    let hundreds = value / 100;
    let remainder = value % 100;
    if remainder == 0 {
        return HUNDREDS[hundreds as usize].to_string();
    }
    format!(
        "{} e {}",
        HUNDREDS[hundreds as usize],
        under_thousand_to_words(remainder)
    )
}
