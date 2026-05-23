use quick_xml::{events::Event, Reader};

use crate::error::ScannerError;
use crate::types::{
    ColorMode, InputCapabilities, ScanCapabilities, ScanFormat, ScanIntent, ScanSettings,
    ScannerState,
};

// ── Namespaces eSCL ────────────────────────────────────────────────────────────

const NS_SCAN: &str = "http://schemas.hp.com/imaging/escl/2011/05/03";
const NS_PWG: &str = "http://www.pwg.org/schemas/2010/12/sm";

// ── Geração de XML ─────────────────────────────────────────────────────────────

/// Gera o XML de ScanSettings para `POST /eSCL/ScanJobs`.
pub fn build_scan_settings_xml(s: &ScanSettings) -> String {
    let duplex = matches!(s.source, crate::types::ScanSource::AdfDuplex);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScanSettings xmlns:scan="{NS_SCAN}" xmlns:pwg="{NS_PWG}">
  <pwg:Version>2.6</pwg:Version>
  <scan:Intent>{intent}</scan:Intent>
  <pwg:ScanRegions>
    <pwg:ScanRegion>
      <pwg:XOffset>{x}</pwg:XOffset>
      <pwg:YOffset>{y}</pwg:YOffset>
      <pwg:Width>{w}</pwg:Width>
      <pwg:Height>{h}</pwg:Height>
      <pwg:ContentRegionUnits>escl:ThreeHundredthsOfInches</pwg:ContentRegionUnits>
    </pwg:ScanRegion>
  </pwg:ScanRegions>
  <scan:InputSource>{source}</scan:InputSource>
  <scan:XResolution>{dpi}</scan:XResolution>
  <scan:YResolution>{dpi}</scan:YResolution>
  <scan:ColorMode>{color}</scan:ColorMode>
  <pwg:DocumentFormat>{mime}</pwg:DocumentFormat>
  <scan:DocumentFormatExt>{mime}</scan:DocumentFormatExt>{duplex_elem}
</scan:ScanSettings>"#,
        intent = s.intent.as_escl(),
        x = s.region.x_offset,
        y = s.region.y_offset,
        w = s.region.width,
        h = s.region.height,
        source = s.source.as_escl(),
        dpi = s.resolution,
        color = s.color_mode.as_escl(),
        mime = s.format.mime_type(),
        duplex_elem = if duplex {
            "\n  <scan:Duplex>true</scan:Duplex>"
        } else {
            ""
        },
    )
}

// ── Parse de XML ──────────────────────────────────────────────────────────────

/// Extrai o nome local de um elemento (strip do prefixo de namespace).
fn local_name(qualified: &[u8]) -> &[u8] {
    qualified
        .iter()
        .position(|&b| b == b':')
        .map(|pos| &qualified[pos + 1..])
        .unwrap_or(qualified)
}

fn local_name_str(qualified: &[u8]) -> String {
    String::from_utf8_lossy(local_name(qualified)).into_owned()
}

/// Parseia `ScannerCapabilities` a partir dos bytes XML da resposta eSCL.
pub fn parse_capabilities(xml: &[u8]) -> Result<ScanCapabilities, ScannerError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut caps = ScanCapabilities::default();
    let mut path: Vec<String> = Vec::new();
    let mut text = String::new();

    let mut platen = InputCapabilities::default();
    let mut adf = InputCapabilities::default();

    #[derive(PartialEq)]
    enum Section { Root, Platen, Adf }
    let mut section = Section::Root;
    let mut current_x_res: Option<u32> = None;

    let mut buf = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| ScannerError::XmlParseError(e.to_string()))?
        {
            Event::Start(e) => {
                let lname = local_name_str(e.name().as_ref());
                match lname.as_str() {
                    "Platen" => section = Section::Platen,
                    "Adf" | "AdfSimplexInputCapabilities" | "AdfDuplexInputCapabilities" => {
                        section = Section::Adf
                    }
                    _ => {}
                }
                path.push(lname);
                text.clear();
            }
            Event::End(e) => {
                let lname = local_name_str(e.name().as_ref());
                let t = text.trim().to_string();

                macro_rules! cur {
                    ($field:ident $op:tt $val:expr) => {
                        match section {
                            Section::Platen => platen.$field $op $val,
                            Section::Adf    => adf.$field $op $val,
                            Section::Root   => {}
                        }
                    };
                }

                match lname.as_str() {
                    "MakeAndModel" => caps.make_model = t,
                    "Version" if path.len() <= 2 => caps.version = t,
                    "MaxWidth" => cur!(max_width = t.parse().unwrap_or(0)),
                    "MaxHeight" => cur!(max_height = t.parse().unwrap_or(0)),
                    "ColorMode" => {
                        if let Some(m) = ColorMode::from_escl(&t) {
                            match section {
                                Section::Platen => platen.supported_color_modes.push(m),
                                Section::Adf    => adf.supported_color_modes.push(m),
                                Section::Root   => {}
                            }
                        }
                    }
                    "DocumentFormat" | "DocumentFormatExt" => {
                        if let Some(f) = ScanFormat::from_mime(&t) {
                            match section {
                                Section::Platen if !platen.supported_formats.contains(&f) => {
                                    platen.supported_formats.push(f)
                                }
                                Section::Adf if !adf.supported_formats.contains(&f) => {
                                    adf.supported_formats.push(f)
                                }
                                _ => {}
                            }
                        }
                    }
                    "XResolution" => current_x_res = t.parse().ok(),
                    "YResolution" => {
                        if let (Some(x), Ok(y)) = (current_x_res, t.parse::<u32>()) {
                            if x == y {
                                match section {
                                    Section::Platen
                                        if !platen.supported_resolutions.contains(&x) =>
                                    {
                                        platen.supported_resolutions.push(x)
                                    }
                                    Section::Adf
                                        if !adf.supported_resolutions.contains(&x) =>
                                    {
                                        adf.supported_resolutions.push(x)
                                    }
                                    _ => {}
                                }
                            }
                        }
                        current_x_res = None;
                    }
                    "SupportedIntent" => {
                        if let Some(i) = ScanIntent::from_escl(&t) {
                            match section {
                                Section::Platen => platen.supported_intents.push(i),
                                Section::Adf    => adf.supported_intents.push(i),
                                Section::Root   => {}
                            }
                        }
                    }
                    "Platen" => {
                        caps.platen = Some(platen.clone());
                        section = Section::Root;
                    }
                    "Adf" => {
                        caps.adf = Some(adf.clone());
                        section = Section::Root;
                    }
                    _ => {}
                }

                path.pop();
                text.clear();
            }
            Event::Text(e) => {
                text = e
                    .unescape()
                    .map(|s| s.into_owned())
                    .unwrap_or_default();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(caps)
}

/// Parseia o estado do scanner de `ScannerStatus`.
pub fn parse_scanner_state(xml: &[u8]) -> Result<ScannerState, ScannerError> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut in_state = false;
    let mut state = ScannerState::Unknown("not found".into());
    let mut buf = Vec::new();

    loop {
        match reader
            .read_event_into(&mut buf)
            .map_err(|e| ScannerError::XmlParseError(e.to_string()))?
        {
            Event::Start(e) => {
                if local_name(e.name().as_ref()) == b"State" {
                    in_state = true;
                }
            }
            Event::Text(e) if in_state => {
                let t = e.unescape().map(|s| s.into_owned()).unwrap_or_default();
                state = ScannerState::from_escl(&t);
                in_state = false;
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(state)
}

// ── Testes ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ScanRegion, ScanSettings, ScanSource};

    fn sample_capabilities_xml() -> &'static [u8] {
        br#"<?xml version="1.0" encoding="UTF-8"?>
<scan:ScannerCapabilities
  xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03"
  xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
  <pwg:Version>2.6</pwg:Version>
  <pwg:MakeAndModel>HP LaserJet MFP M234sdwe</pwg:MakeAndModel>
  <scan:Platen>
    <scan:InputCapabilities>
      <scan:MaxWidth>2480</scan:MaxWidth>
      <scan:MaxHeight>3508</scan:MaxHeight>
      <scan:SettingProfiles>
        <scan:SettingProfile>
          <scan:ColorModes>
            <scan:ColorMode>BlackAndWhite1</scan:ColorMode>
            <scan:ColorMode>Grayscale8</scan:ColorMode>
            <scan:ColorMode>RGB24</scan:ColorMode>
          </scan:ColorModes>
          <scan:DocumentFormats>
            <pwg:DocumentFormat>application/pdf</pwg:DocumentFormat>
            <pwg:DocumentFormat>image/jpeg</pwg:DocumentFormat>
          </scan:DocumentFormats>
          <scan:SupportedResolutions>
            <scan:DiscreteResolutions>
              <scan:DiscreteResolution>
                <scan:XResolution>75</scan:XResolution>
                <scan:YResolution>75</scan:YResolution>
              </scan:DiscreteResolution>
              <scan:DiscreteResolution>
                <scan:XResolution>300</scan:XResolution>
                <scan:YResolution>300</scan:YResolution>
              </scan:DiscreteResolution>
              <scan:DiscreteResolution>
                <scan:XResolution>600</scan:XResolution>
                <scan:YResolution>600</scan:YResolution>
              </scan:DiscreteResolution>
            </scan:DiscreteResolutions>
          </scan:SupportedResolutions>
        </scan:SettingProfile>
      </scan:SettingProfiles>
      <scan:SupportedIntents>
        <scan:SupportedIntent>Document</scan:SupportedIntent>
        <scan:SupportedIntent>Photo</scan:SupportedIntent>
      </scan:SupportedIntents>
    </scan:InputCapabilities>
  </scan:Platen>
</scan:ScannerCapabilities>"#
    }

    #[test]
    fn parse_capabilities_extrai_make_model() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        assert_eq!(caps.make_model, "HP LaserJet MFP M234sdwe");
    }

    #[test]
    fn parse_capabilities_extrai_platen() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        let platen = caps.platen.expect("platen deve estar presente");
        assert_eq!(platen.max_width, 2480);
        assert_eq!(platen.max_height, 3508);
    }

    #[test]
    fn parse_capabilities_extrai_formatos() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        let p = caps.platen.unwrap();
        assert!(p.supports_format(&ScanFormat::Pdf));
        assert!(p.supports_format(&ScanFormat::Jpeg));
        assert!(!p.supports_format(&ScanFormat::Png));
    }

    #[test]
    fn parse_capabilities_extrai_resolucoes() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        let p = caps.platen.unwrap();
        assert!(p.supports_resolution(300));
        assert!(p.supports_resolution(600));
        assert!(!p.supports_resolution(200));
    }

    #[test]
    fn parse_capabilities_extrai_color_modes() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        let p = caps.platen.unwrap();
        assert!(p.supports_color_mode(&ColorMode::Grayscale8));
        assert!(p.supports_color_mode(&ColorMode::Rgb24));
        assert!(p.supports_color_mode(&ColorMode::BlackAndWhite1));
    }

    #[test]
    fn parse_capabilities_extrai_intents() {
        let caps = parse_capabilities(sample_capabilities_xml()).unwrap();
        let p = caps.platen.unwrap();
        assert!(p.supported_intents.contains(&ScanIntent::Document));
        assert!(p.supported_intents.contains(&ScanIntent::Photo));
    }

    #[test]
    fn build_scan_settings_xml_contem_campos_obrigatorios() {
        let s = ScanSettings {
            source: ScanSource::Platen,
            format: ScanFormat::Pdf,
            intent: ScanIntent::Document,
            color_mode: ColorMode::Grayscale8,
            resolution: 300,
            region: ScanRegion::A4_PORTRAIT,
        };
        let xml = build_scan_settings_xml(&s);
        assert!(xml.contains("<scan:Intent>Document</scan:Intent>"));
        assert!(xml.contains("<scan:InputSource>Platen</scan:InputSource>"));
        assert!(xml.contains("<scan:XResolution>300</scan:XResolution>"));
        assert!(xml.contains("<scan:ColorMode>Grayscale8</scan:ColorMode>"));
        assert!(xml.contains("<pwg:DocumentFormat>application/pdf</pwg:DocumentFormat>"));
        assert!(xml.contains("<pwg:Width>2480</pwg:Width>"));
        assert!(xml.contains("<pwg:Height>3508</pwg:Height>"));
    }

    #[test]
    fn build_scan_settings_xml_jpeg_cor() {
        let s = ScanSettings {
            format: ScanFormat::Jpeg,
            color_mode: ColorMode::Rgb24,
            resolution: 600,
            ..ScanSettings::default()
        };
        let xml = build_scan_settings_xml(&s);
        assert!(xml.contains("image/jpeg"));
        assert!(xml.contains("RGB24"));
        assert!(xml.contains("<scan:XResolution>600</scan:XResolution>"));
    }

    #[test]
    fn parse_scanner_state_idle() {
        let xml = br#"<?xml version="1.0"?>
<scan:ScannerStatus xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03"
                    xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
  <pwg:Version>2.6</pwg:Version>
  <pwg:State>Idle</pwg:State>
</scan:ScannerStatus>"#;
        let state = parse_scanner_state(xml).unwrap();
        assert_eq!(state, ScannerState::Idle);
        assert!(state.is_ready());
    }

    #[test]
    fn from_mime_ignora_parametros_content_type() {
        use crate::types::ScanFormat;
        assert_eq!(ScanFormat::from_mime("application/pdf"), Some(ScanFormat::Pdf));
        assert_eq!(ScanFormat::from_mime("image/jpeg; charset=utf-8"), Some(ScanFormat::Jpeg));
        assert_eq!(ScanFormat::from_mime("image/tiff; boundary=xxx"), Some(ScanFormat::Tiff));
        assert_eq!(ScanFormat::from_mime("image/png"), Some(ScanFormat::Png));
        assert_eq!(ScanFormat::from_mime("text/html"), None);
    }

    #[test]
    fn parse_scanner_state_processing() {
        let xml = br#"<?xml version="1.0"?>
<scan:ScannerStatus xmlns:scan="http://schemas.hp.com/imaging/escl/2011/05/03"
                    xmlns:pwg="http://www.pwg.org/schemas/2010/12/sm">
  <pwg:State>Processing</pwg:State>
</scan:ScannerStatus>"#;
        let state = parse_scanner_state(xml).unwrap();
        assert_eq!(state, ScannerState::Processing);
        assert!(!state.is_ready());
    }
}
