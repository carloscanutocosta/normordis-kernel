use normordis_pdf::{FontFamily, FontRegistry};

#[test]
fn default_registry_does_not_panic() {
    let _reg = FontRegistry::default();
}

#[test]
fn measure_single_char_is_positive() {
    let reg = FontRegistry::default();
    let w = reg.measure_text_mm("A", "LibertinusSerif", 12.0, false, false);
    assert!(w > 0.0, "expected positive width, got {w}");
}

#[test]
fn measure_empty_string_is_zero() {
    let reg = FontRegistry::default();
    let w = reg.measure_text_mm("", "LibertinusSerif", 12.0, false, false);
    assert_eq!(w, 0.0);
}

#[test]
fn measure_two_chars_wider_than_one() {
    let reg = FontRegistry::default();
    let w1 = reg.measure_text_mm("A", "LibertinusSerif", 12.0, false, false);
    let w2 = reg.measure_text_mm("AA", "LibertinusSerif", 12.0, false, false);
    assert!(w2 > w1, "two chars should be wider than one: {w2} vs {w1}");
}

#[test]
fn get_variant_bold_returns_font() {
    let reg = FontRegistry::default();
    let family = reg.get_default();
    let bold_variant = family.get_variant(true, false);
    assert!(bold_variant.measure_text_mm("X", 12.0) > 0.0);
}

#[test]
fn default_family_name_is_liberation_sans() {
    let reg = FontRegistry::default();
    assert_eq!(reg.default_family_name(), "LiberationSans");
}

#[test]
fn space_char_has_positive_advance() {
    let reg = FontRegistry::default();
    let w = reg.measure_text_mm(" ", "LibertinusSerif", 11.0, false, false);
    assert!(w > 0.5, "space advance should be > 0.5 mm at 11pt, got {w}");
}

#[test]
fn portuguese_chars_have_positive_advance() {
    let reg = FontRegistry::default();
    for ch in ['ã', 'ç', 'é', 'ó', 'ú', 'â', 'ê', 'ô', 'à'] {
        let w = reg.measure_text_mm(&ch.to_string(), "LibertinusSerif", 11.0, false, false);
        assert!(w > 0.0, "char '{ch}' should have positive advance, got {w}");
    }
}

#[test]
fn from_dir_loads_libertinus_from_assets() {
    let dir = std::path::Path::new("assets/fonts");
    let reg = FontRegistry::from_dir(dir).expect("should load fonts from assets/fonts");
    assert!(!reg.default_family_name().is_empty());
    let w = reg.measure_text_mm("A", reg.default_family_name(), 12.0, false, false);
    assert!(w > 0.0, "loaded font should measure text, got {w}");
}

#[test]
fn from_dir_missing_directory_returns_err() {
    let dir = std::path::Path::new("assets/nonexistent_dir_xyz");
    let result = FontRegistry::from_dir(dir);
    assert!(result.is_err(), "missing directory should return Err");
}

#[test]
fn liberation_sans_family_loads_ok() {
    let fam = normordis_pdf::liberation_sans_family().expect("should build LiberationSans family");
    assert_eq!(fam.name, "LiberationSans");
    let w = fam.measure_text_mm("X", 12.0, false, false);
    assert!(w > 0.0);
}

#[test]
fn font_family_from_bytes_invalid_returns_err() {
    let result = FontFamily::from_bytes("Bad", vec![0u8; 64], None, None, None);
    assert!(result.is_err(), "garbage bytes should fail to parse");
}
