use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum NormalizationError {
    #[error("data invalida: {0}")]
    InvalidDate(String),
    #[error("valor monetario invalido")]
    InvalidMoney,
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

pub fn strip_diacritics(value: &str) -> String {
    value.chars().map(replace_diacritic).collect::<String>()
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
    let cleaned = normalize_number_string(value)?;
    cleaned.parse::<i64>().ok()
}

pub fn parse_f64_loose(value: &str) -> Option<f64> {
    let cleaned = normalize_number_string(value)?;
    cleaned.parse::<f64>().ok()
}

pub fn round_to_places(value: f64, places: u32) -> f64 {
    let factor = 10_f64.powi(places as i32);
    (value * factor).round() / factor
}

pub fn money_to_cents(value: f64) -> Result<i64, NormalizationError> {
    if !value.is_finite() {
        return Err(NormalizationError::InvalidMoney);
    }
    let cents = (value * 100.0).round();
    if cents < i64::MIN as f64 || cents > i64::MAX as f64 {
        return Err(NormalizationError::InvalidMoney);
    }
    Ok(cents as i64)
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

pub fn money_cents_to_words_eur(cents: i64) -> Result<String, NormalizationError> {
    let abs = cents.unsigned_abs();
    let euros = abs / 100;
    let centimos = abs % 100;
    if euros > 999_999_999_999 {
        return Err(NormalizationError::NumberOutOfRange);
    }

    let euros_words = if euros == 1 {
        "um euro".to_string()
    } else {
        format!("{} euros", number_to_words_positive(euros))
    };

    let mut result = if cents < 0 {
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
    if local.is_empty() || domain.is_empty() || value.matches('@').count() != 1 {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') || !domain.contains('.') {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    domain
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        && local
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-'))
}

fn normalize_number_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut result = String::new();
    let mut seen_separator = false;
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx == 0 && matches!(ch, '+' | '-') {
            result.push(ch);
            continue;
        }
        if ch.is_ascii_digit() {
            result.push(ch);
            continue;
        }
        if matches!(ch, '.' | ',') {
            if seen_separator {
                continue;
            }
            seen_separator = true;
            result.push('.');
        }
    }

    if result.is_empty() || result == "+" || result == "-" {
        None
    } else {
        Some(result)
    }
}

fn replace_diacritic(ch: char) -> char {
    match ch {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'u',
        'ç' | 'Ç' => 'c',
        'ñ' | 'Ñ' => 'n',
        _ => ch,
    }
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
