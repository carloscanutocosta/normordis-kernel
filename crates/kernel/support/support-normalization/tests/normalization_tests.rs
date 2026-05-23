use support_normalization::{
    capitalize_first, digits_only, is_valid_email, is_valid_nif, money_to_words_eur,
    normalize_date_to_iso, normalize_for_lookup, normalize_portuguese_name, normalize_whitespace,
    number_to_words_pt, parse_f64_loose, parse_i64_loose, round_to_places, strip_diacritics,
    title_case, trim_to_none,
};

#[test]
fn normalizes_whitespace_and_trim() {
    assert_eq!(normalize_whitespace("  Rua   do   Ouro "), "Rua do Ouro");
    assert_eq!(trim_to_none("   "), None);
    assert_eq!(trim_to_none("  teste "), Some("teste".to_string()));
}

#[test]
fn strips_diacritics_and_normalizes_lookup() {
    assert_eq!(strip_diacritics("Órgão Público"), "orgao Publico");
    assert_eq!(normalize_for_lookup("  Órgão   Público "), "orgao publico");
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
    assert_eq!(round_to_places(12.3456, 2), 12.35);
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
fn converts_money_to_words() {
    assert_eq!(
        money_to_words_eur(1234.56).unwrap(),
        "mil, duzentos e trinta e quatro euros e cinquenta e seis cêntimos"
    );
    assert_eq!(money_to_words_eur(1.0).unwrap(), "um euro");
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
}
