use support_address::{parse_postal_code, AddressCandidate};

#[test]
fn parses_valid_postal_code() {
    let postal_code = parse_postal_code("4700-001").unwrap();
    assert_eq!(postal_code.cp4, "4700");
    assert_eq!(postal_code.cp3, "001");
}

#[test]
fn rejects_invalid_postal_code() {
    assert!(parse_postal_code("4700").is_err());
    assert!(parse_postal_code("47A0-001").is_err());
}

#[test]
fn can_build_display_label() {
    assert_eq!(
        sample_candidate().display_label(),
        "Rua Costa Soares, Braga"
    );
}

#[test]
fn formats_postal_address_ignoring_empty_fields() {
    assert_eq!(
        sample_candidate().format_postal_address(),
        "Rua Costa Soares, Braga\r\n4700-001 Braga"
    );
}

fn sample_candidate() -> AddressCandidate {
    AddressCandidate {
        postal_code_id: 1,
        postal_code: "4700-001".to_string(),
        postal_designation: Some("Braga".to_string()),
        locality_name: Some("Braga".to_string()),
        artery_type: Some("Rua".to_string()),
        artery_title: None,
        artery_name: Some("Costa Soares".to_string()),
        artery_local: None,
        section: None,
    }
}
