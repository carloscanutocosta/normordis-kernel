use rust_decimal::Decimal;
use support_normalization::{
    capitalize_first, digits_only, is_valid_email, is_valid_nif, letters_and_digits_only,
    money_cents_to_words_eur, money_decimal_to_cents, money_str_to_cents, money_str_to_words_eur,
    money_to_words_eur, normalize_date_to_iso, normalize_domain_to_ascii, normalize_for_lookup,
    normalize_portuguese_name, normalize_unicode_nfc, normalize_unicode_nfd, normalize_unicode_nfkc,
    normalize_unicode_nfkd, normalize_whitespace, number_to_words_pt, parse_f64_loose,
    parse_i64_loose, round_to_places, strip_diacritics, title_case, trim_to_none,
};

#[test]
fn normalizes_whitespace_and_trim() {
    assert_eq!(normalize_whitespace("  Rua   do   Ouro "), "Rua do Ouro");
    assert_eq!(trim_to_none("   "), None);
    assert_eq!(trim_to_none("  teste "), Some("teste".to_string()));
}

#[test]
fn strips_diacritics_and_normalizes_lookup() {
    assert_eq!(strip_diacritics("Órgão Público"), "Orgao Publico");
    assert_eq!(strip_diacritics("A\u{0301}gua"), "Agua");
    assert_eq!(normalize_for_lookup("  Órgão   Público "), "orgao publico");
}

#[test]
fn normalizes_unicode_forms() {
    assert_eq!(normalize_unicode_nfc("A\u{0301}"), "Á");
    assert_eq!(normalize_unicode_nfd("Á"), "A\u{0301}");
    assert_eq!(normalize_unicode_nfkc("１２３"), "123");
    assert_eq!(normalize_unicode_nfkd("①"), "1");
}

#[test]
fn capitalizes_text() {
    assert_eq!(capitalize_first("teste"), "Teste");
    assert_eq!(title_case("rUA dO ouRO"), "Rua Do Ouro");
    assert_eq!(
        normalize_portuguese_name("jOAO da silva e costa"),
        "Joao da Silva e Costa"
    );
    assert_eq!(
        normalize_portuguese_name("maria do carmo ferreira-gomes"),
        "Maria do Carmo Ferreira-Gomes"
    );
}

#[test]
fn filters_and_parses_numbers() {
    assert_eq!(digits_only("PT 123 456 789"), "123456789");
    assert_eq!(parse_i64_loose("42"), Some(42));
    assert_eq!(parse_f64_loose(" 123,45 "), Some(123.45));
    assert_eq!(parse_i64_loose("1 234 567"), Some(1_234_567));
    assert_eq!(parse_i64_loose("1.234.567"), Some(1_234_567));
    assert_eq!(parse_i64_loose("abc123"), None);
    assert_eq!(parse_i64_loose("12.34"), None);
    assert_eq!(parse_f64_loose("1.234,56"), Some(1234.56));
    assert_eq!(parse_f64_loose("1,234.56"), Some(1234.56));
    assert_eq!(parse_f64_loose("1 234,56"), Some(1234.56));
    assert_eq!(parse_f64_loose("12..34"), None);
    assert_eq!(parse_f64_loose("1,23,4"), None);
    assert_eq!(round_to_places(12.3456, 2), 12.35);
}

#[test]
fn letters_and_digits_only_filters_correctly() {
    assert_eq!(letters_and_digits_only("Rua 123 €!"), "Rua123");
    assert_eq!(letters_and_digits_only("abc-def_123"), "abcdef123");
    assert_eq!(letters_and_digits_only(""), "");
    assert_eq!(letters_and_digits_only("café"), "café");
}

#[test]
fn converts_numbers_to_words() {
    assert_eq!(number_to_words_pt(0).unwrap(), "zero");
    assert_eq!(number_to_words_pt(21).unwrap(), "vinte e um");
    assert_eq!(
        number_to_words_pt(1_245).unwrap(),
        "mil, duzentos e quarenta e cinco"
    );
}

#[test]
fn number_to_words_pt_edge_cases() {
    assert_eq!(number_to_words_pt(100).unwrap(), "cem");
    assert_eq!(number_to_words_pt(-5).unwrap(), "menos cinco");
    assert_eq!(number_to_words_pt(1_001).unwrap(), "mil e um");
    assert_eq!(number_to_words_pt(1_000_000).unwrap(), "um milhão");
    assert_eq!(number_to_words_pt(2_000_000).unwrap(), "dois milhões");
    assert!(number_to_words_pt(1_000_000_000_000).is_err());
    assert!(number_to_words_pt(-1_000_000_000_000).is_err());
}

#[test]
fn converts_money_to_words() {
    assert_eq!(
        money_to_words_eur(1234.56).unwrap(),
        "mil, duzentos e trinta e quatro euros e cinquenta e seis cêntimos"
    );
    assert_eq!(money_to_words_eur(1.0).unwrap(), "um euro");
    assert_eq!(money_str_to_cents("1.234,56").unwrap(), 123_456);
    assert_eq!(money_str_to_cents("-1,05").unwrap(), -105);
    assert!(money_str_to_cents("1,234").is_err());
    assert!(money_str_to_cents("1.234").is_err());
    assert!(money_str_to_cents("1,2345").is_err());
    assert_eq!(
        money_str_to_words_eur("1234,56").unwrap(),
        "mil, duzentos e trinta e quatro euros e cinquenta e seis cêntimos"
    );
}

#[test]
fn money_cents_to_words_eur_edge_cases() {
    assert_eq!(money_cents_to_words_eur(0).unwrap(), "zero euros");
    assert_eq!(money_cents_to_words_eur(1).unwrap(), "um cêntimo");
    assert_eq!(money_cents_to_words_eur(99).unwrap(), "noventa e nove cêntimos");
    assert_eq!(money_cents_to_words_eur(-5).unwrap(), "menos cinco cêntimos");
    assert_eq!(money_cents_to_words_eur(100).unwrap(), "um euro");
    assert_eq!(money_cents_to_words_eur(200).unwrap(), "dois euros");
    assert_eq!(
        money_cents_to_words_eur(-105).unwrap(),
        "menos um euro e cinco cêntimos"
    );
    assert!(money_cents_to_words_eur(100_000_000_000_000).is_err());
}

#[test]
fn money_decimal_to_cents_exact() {
    assert_eq!(money_decimal_to_cents(Decimal::new(12356, 2)).unwrap(), 12356);
    assert_eq!(money_decimal_to_cents(Decimal::new(-100, 2)).unwrap(), -100);
    assert_eq!(money_decimal_to_cents(Decimal::new(0, 0)).unwrap(), 0);
    assert!(money_decimal_to_cents(Decimal::new(12356, 3)).is_err());
}

#[test]
fn normalizes_dates_to_iso() {
    assert_eq!(normalize_date_to_iso("25/03/2026").unwrap(), "2026-03-25");
    assert_eq!(normalize_date_to_iso("2026-03-25").unwrap(), "2026-03-25");
    assert!(normalize_date_to_iso("2026/25/03").is_err());
}

#[test]
fn validates_nif_and_email() {
    assert!(is_valid_nif("501964843"));
    assert!(!is_valid_nif("501964842"));
    assert!(is_valid_email("user.name+tag@example.pt"));
    assert!(!is_valid_email("user@@example.pt"));
    assert!(!is_valid_email("user@example..pt"));
    assert!(!is_valid_email("user@-example.pt"));
    assert!(!is_valid_email("user@example-.pt"));
    assert!(!is_valid_email("user@example.p"));
    assert!(!is_valid_email(".user@example.pt"));
    assert!(is_valid_email("user@exemplo.pt"));
    assert!(is_valid_email("user@exemplo.рф"));
    assert_eq!(
        normalize_domain_to_ascii("Exemplo.PT").as_deref(),
        Some("exemplo.pt")
    );
    assert!(normalize_domain_to_ascii("exemplo.рф")
        .is_some_and(|domain| domain.starts_with("exemplo.xn--")));
}

#[test]
fn nif_edge_cases() {
    assert!(!is_valid_nif(""));
    assert!(!is_valid_nif("12345678"));
    assert!(!is_valid_nif("1234567890"));
    assert!(!is_valid_nif("400000000"));
    assert!(!is_valid_email(&format!("{}@example.pt", "a".repeat(65))));
}
