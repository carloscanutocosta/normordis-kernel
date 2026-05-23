use std::io::Read;
use std::time::Instant;

use native_tls::TlsConnector;

use crate::error::ScannerError;
use crate::types::{
    ScanCapabilities, ScanSettings, ScannerClientConfig, ScannerDevice, ScannerState, ScannedDocument,
};
use crate::xml::{build_scan_settings_xml, parse_capabilities, parse_scanner_state};

/// eSCL client for network document scanning.
///
/// Create with `ScannerClient::new(device, config)` and reuse across multiple scans.
pub struct ScannerClient {
    device: ScannerDevice,
    config: ScannerClientConfig,
    agent: ureq::Agent,
}

impl ScannerClient {
    pub fn new(device: ScannerDevice, config: ScannerClientConfig) -> Self {
        let mut builder = ureq::AgentBuilder::new()
            .timeout_connect(config.connect_timeout)
            .timeout_read(config.job_timeout);

        if device.danger_accept_invalid_certs {
            let connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .expect("invalid TLS connector");
            builder = builder.tls_connector(std::sync::Arc::new(connector));
        }

        Self {
            device,
            config,
            agent: builder.build(),
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Queries scanner capabilities (formats, resolutions, sources).
    pub fn capabilities(&self) -> Result<ScanCapabilities, ScannerError> {
        let url = self.device.escl_url("/ScannerCapabilities");
        let xml = self.get_xml(&url)?;
        parse_capabilities(xml.as_bytes())
    }

    /// Queries current scanner state.
    pub fn status(&self) -> Result<ScannerState, ScannerError> {
        let url = self.device.escl_url("/ScannerStatus");
        let xml = self.get_xml(&url)?;
        parse_scanner_state(xml.as_bytes())
    }

    /// Executes a full scan: create job → wait for document → return bytes.
    ///
    /// Blocks until the document is available or `config.job_timeout` expires.
    pub fn scan(&self, settings: &ScanSettings) -> Result<ScannedDocument, ScannerError> {
        self.validate_settings(settings)?;
        let job_uri = self.create_job(settings)?;
        let doc_url = format!("{}/NextDocument", job_uri);
        self.fetch_document_with_retry(&doc_url)
    }

    // ── Internals ─────────────────────────────────────────────────────────────

    fn validate_settings(&self, s: &ScanSettings) -> Result<(), ScannerError> {
        if s.resolution == 0 {
            return Err(ScannerError::InvalidConfig("resolution must be > 0".into()));
        }
        if s.region.width == 0 || s.region.height == 0 {
            return Err(ScannerError::InvalidConfig("region width and height must be > 0".into()));
        }
        Ok(())
    }

    fn create_job(&self, settings: &ScanSettings) -> Result<String, ScannerError> {
        let url = self.device.escl_url("/ScanJobs");
        let body = build_scan_settings_xml(settings);

        let resp = self
            .agent
            .post(&url)
            .set("Content-Type", "text/xml; charset=UTF-8")
            .send_string(&body)
            .map_err(|e| match e {
                ureq::Error::Status(code, r) => ScannerError::HttpError {
                    status: code,
                    message: r.into_string().unwrap_or_default(),
                },
                ureq::Error::Transport(t) => ScannerError::NetworkError(t.to_string()),
            })?;

        let location = resp
            .header("Location")
            .ok_or_else(|| ScannerError::JobFailed {
                reason: "201 response missing Location header".into(),
            })?;

        Ok(self.resolve_uri(location))
    }

    fn fetch_document_with_retry(&self, url: &str) -> Result<ScannedDocument, ScannerError> {
        let deadline = Instant::now() + self.config.job_timeout;
        let start = Instant::now();
        let mut last_progress = Instant::now();

        eprintln!("Waiting for scanner...");

        loop {
            if Instant::now() >= deadline {
                return Err(ScannerError::Timeout {
                    timeout_secs: self.config.job_timeout.as_secs(),
                });
            }

            match self.try_fetch_document(url) {
                Ok(doc) => return Ok(doc),
                Err(ScannerError::NotReady) => {
                    if last_progress.elapsed().as_secs() >= 5 {
                        eprintln!("  Still scanning... ({:.0}s elapsed)", start.elapsed().as_secs_f32());
                        last_progress = Instant::now();
                    }
                    std::thread::sleep(self.config.poll_interval);
                }
                Err(e) => return Err(e),
            }
        }
    }

    fn try_fetch_document(&self, url: &str) -> Result<ScannedDocument, ScannerError> {
        match self.agent.get(url).call() {
            Ok(resp) => {
                let content_type = resp
                    .header("Content-Type")
                    .unwrap_or("application/octet-stream")
                    .to_string();

                // Some devices (e.g. Epson) return 200 OK with an XML status body
                // while the job is still processing. Treat XML responses as not-ready.
                let ct_base = content_type.split(';').next().unwrap_or("").trim();
                if ct_base == "application/xml" || ct_base == "text/xml" {
                    return Err(ScannerError::NotReady);
                }

                let format = crate::types::ScanFormat::from_mime(&content_type)
                    .ok_or_else(|| ScannerError::FormatNotSupported {
                        format: content_type.clone(),
                    })?;

                eprintln!("  Downloading document ({content_type})...");
                let mut data = Vec::new();
                resp.into_reader()
                    .read_to_end(&mut data)
                    .map_err(|e| ScannerError::NetworkError(e.to_string()))?;

                Ok(ScannedDocument { format, data, content_type })
            }
            // 200 with no content-type or empty body treated as not-ready
            Err(ureq::Error::Status(503, _)) | Err(ureq::Error::Status(202, _)) => {
                Err(ScannerError::NotReady)
            }
            Err(ureq::Error::Status(404, _)) | Err(ureq::Error::Status(410, _)) => {
                Err(ScannerError::JobFailed {
                    reason: "document not found (job aborted or expired)".into(),
                })
            }
            Err(ureq::Error::Status(code, r)) => Err(ScannerError::HttpError {
                status: code,
                message: r.into_string().unwrap_or_default(),
            }),
            Err(ureq::Error::Transport(_)) => Err(ScannerError::NetworkError(
                "connection lost while waiting for document".into(),
            )),
        }
    }

    fn get_xml(&self, url: &str) -> Result<String, ScannerError> {
        match self.agent.get(url).set("Accept", "application/xml, text/xml").call() {
            Ok(resp) => resp
                .into_string()
                .map_err(|e| ScannerError::NetworkError(e.to_string())),
            Err(ureq::Error::Status(404, _)) => Err(ScannerError::HttpError {
                status: 404,
                message: format!(
                    "eSCL endpoint not found (path: {}). Try a different base path (e.g. /ESCL or /eSCL).",
                    self.device.base_path
                ),
            }),
            Err(ureq::Error::Status(code, r)) => {
                let msg = r.into_string().unwrap_or_default();
                Err(ScannerError::HttpError { status: code, message: msg })
            }
            Err(ureq::Error::Transport(_t)) => Err(ScannerError::DeviceNotFound {
                host: self.device.host.clone(),
                port: self.device.port,
            }),
        }
    }

    /// Resolves a relative URI to absolute using the device base_url.
    ///
    /// Some devices (e.g. Epson) return `http://` in the Location header even
    /// when the job was created via HTTPS. Upgrade the scheme when necessary.
    fn resolve_uri(&self, location: &str) -> String {
        if location.starts_with("https://") {
            location.to_string()
        } else if location.starts_with("http://") {
            if self.device.uses_https {
                location.replacen("http://", "https://", 1)
            } else {
                location.to_string()
            }
        } else {
            format!("{}{}", self.device.base_url(), location)
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_device() -> ScannerDevice {
        ScannerDevice::http("192.168.1.100", 80)
    }

    #[test]
    fn escl_url_builds_correctly() {
        let d = sample_device();
        assert_eq!(
            d.escl_url("/ScannerCapabilities"),
            "http://192.168.1.100:80/eSCL/ScannerCapabilities"
        );
        assert_eq!(d.escl_url("/ScanJobs"), "http://192.168.1.100:80/eSCL/ScanJobs");
    }

    #[test]
    fn resolve_relative_uri() {
        let client = ScannerClient::new(sample_device(), ScannerClientConfig::default());
        let resolved = client.resolve_uri("/eSCL/ScanJobs/42");
        assert_eq!(resolved, "http://192.168.1.100:80/eSCL/ScanJobs/42");
    }

    #[test]
    fn resolve_absolute_uri_unchanged() {
        let client = ScannerClient::new(sample_device(), ScannerClientConfig::default());
        let abs = "http://192.168.1.100:80/eSCL/ScanJobs/42";
        assert_eq!(client.resolve_uri(abs), abs);
    }

    #[test]
    fn resolve_uri_upgrades_http_to_https_for_tls_device() {
        let device = ScannerDevice {
            name: "Epson ET-2750".into(),
            host: "192.168.1.82".into(),
            port: 443,
            uses_https: true,
            base_path: "/eSCL".into(),
            danger_accept_invalid_certs: true,
        };
        let client = ScannerClient::new(device, ScannerClientConfig::default());
        // Epson returns http:// in Location even when the job was created via HTTPS
        let location = "http://192.168.1.82/eSCL/ScanJobs/385af202/NextDocument";
        assert_eq!(
            client.resolve_uri(location),
            "https://192.168.1.82/eSCL/ScanJobs/385af202/NextDocument"
        );
    }

    #[test]
    fn validate_settings_rejects_zero_resolution() {
        let client = ScannerClient::new(sample_device(), ScannerClientConfig::default());
        let mut s = ScanSettings::default();
        s.resolution = 0;
        let err = client.validate_settings(&s).unwrap_err();
        assert!(matches!(err, ScannerError::InvalidConfig(_)));
    }

    #[test]
    fn validate_settings_rejects_zero_region() {
        let client = ScannerClient::new(sample_device(), ScannerClientConfig::default());
        let mut s = ScanSettings::default();
        s.region.width = 0;
        let err = client.validate_settings(&s).unwrap_err();
        assert!(matches!(err, ScannerError::InvalidConfig(_)));
    }

    #[test]
    fn scanner_device_https() {
        let d = ScannerDevice {
            name: "Lexmark MX431".into(),
            host: "192.168.1.50".into(),
            port: 443,
            uses_https: true,
            base_path: "/eSCL".into(),
            danger_accept_invalid_certs: false,
        };
        assert_eq!(d.base_url(), "https://192.168.1.50:443");
        assert_eq!(
            d.escl_url("/ScannerCapabilities"),
            "https://192.168.1.50:443/eSCL/ScannerCapabilities"
        );
    }

    // ── Integration tests (require a physical scanner on the network) ──────────

    #[test]
    #[ignore = "requires eSCL scanner at 192.168.1.100:80"]
    fn integration_capabilities_hp() {
        let client = ScannerClient::new(ScannerDevice::http("192.168.1.100", 80), ScannerClientConfig::default());
        let caps = client.capabilities().unwrap();
        assert!(!caps.make_model.is_empty());
        assert!(caps.platen.is_some());
    }

    #[test]
    #[ignore = "requires eSCL scanner at 192.168.1.100:80"]
    fn integration_scan_a4_pdf_300dpi() {
        let client = ScannerClient::new(ScannerDevice::http("192.168.1.100", 80), ScannerClientConfig::default());
        let doc = client.scan(&ScanSettings::default()).unwrap();
        assert!(!doc.data.is_empty());
        assert_eq!(doc.format, crate::types::ScanFormat::Pdf);
        assert!(doc.data.starts_with(b"%PDF"));
    }
}
