use serde::{Deserialize, Serialize};

use crate::{NormaxisPdfError, Result};

/// Options for preparing a PDF for digital signature.
///
/// Pass to [`DocumentBuilder::render_prepared_for_signing`] to produce a
/// [`PreparedPdf`] whose byte ranges can then be signed externally and
/// embedded back with [`PreparedPdf::embed_signature`].
#[derive(Debug, Clone)]
pub struct SignatureOptions {
    /// Signing reason text (appears in the signature field).
    pub reason: String,
    /// Geographical signing location.
    pub location: String,
    /// Bytes reserved for the PKCS#7 DER blob (default 8192).
    /// Must be ≥ the actual PKCS#7 size produced by the signing key.
    pub reserved_bytes: usize,
}

impl Default for SignatureOptions {
    fn default() -> Self {
        Self {
            reason: String::from("Assinado digitalmente"),
            location: String::from("Portugal"),
            reserved_bytes: 8192,
        }
    }
}

/// A rendered PDF ready for external signing.
///
/// The document has a reserved `/Contents` placeholder and a `/ByteRange`
/// that excludes that placeholder.  Sign the data returned by
/// [`bytes_to_sign`] and pass the DER-encoded result to
/// [`embed_signature`] to produce the final PDF.
pub struct PreparedPdf {
    pub(crate) bytes: Vec<u8>,
    /// Byte offset of the `<` that opens the `/Contents` hex string.
    pub(crate) contents_start: usize,
    /// Number of bytes reserved for the PKCS#7 blob.
    pub(crate) reserved_bytes: usize,
}

impl PreparedPdf {
    /// The raw PDF bytes (with ByteRange patched, Contents still zeroed).
    pub fn raw_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// `(range1_start, range1_len, range2_start, range2_len)` as specified
    /// in the PDF `/ByteRange` array.
    pub fn byte_range(&self) -> (u64, u64, u64, u64) {
        let a = self.contents_start as u64;
        let b = a + 1 + self.reserved_bytes as u64 * 2 + 1; // past '>'
        (0, a, b, self.bytes.len() as u64 - b)
    }

    /// Concatenation of the two signed byte ranges — pass this to the
    /// signing algorithm (SHA-256 / SHA-512 as required by the key type).
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        let (_, r1_len, r2_start, r2_len) = self.byte_range();
        let r1 = &self.bytes[..r1_len as usize];
        let r2 = &self.bytes[r2_start as usize..r2_start as usize + r2_len as usize];
        [r1, r2].concat()
    }

    /// Embed a DER-encoded PKCS#7/CMS signature and return the signed PDF.
    ///
    /// `pkcs7_der` must be no larger than `SignatureOptions::reserved_bytes`.
    pub fn embed_signature(mut self, pkcs7_der: &[u8]) -> Result<Vec<u8>> {
        if pkcs7_der.len() > self.reserved_bytes {
            return Err(NormaxisPdfError::RenderError(format!(
                "PKCS#7 blob ({} B) exceeds reserved space ({} B)",
                pkcs7_der.len(),
                self.reserved_bytes,
            )));
        }
        let hex: String = pkcs7_der.iter().map(|b| format!("{b:02x}")).collect();
        let pos = self.contents_start + 1; // skip '<'
        self.bytes[pos..pos + hex.len()].copy_from_slice(hex.as_bytes());
        // Pad remaining reserved space with '0' hex digits (represents null bytes)
        let fill_end = pos + self.reserved_bytes * 2;
        self.bytes[pos + hex.len()..fill_end].fill(b'0');
        Ok(self.bytes)
    }
}

// ── SignatureConfig / SignatureField ──────────────────────────────────────────

/// Visual signature field position on a specific page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureField {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    /// 1-based page number where the field appears.
    pub page: u32,
    /// Label displayed above the signature line.
    pub label: String,
}

/// High-level configuration for digital signature preparation.
///
/// Used with [`DocumentBuilder::sign`] or [`sign_pdf`].
/// The actual PKCS#7/CMS blob must be produced externally (HSM, qualified
/// signing service, or a Rust crypto stack such as `p256` + `x509-cert`).
///
/// [`DocumentBuilder::sign`]: crate::DocumentBuilder::sign
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignatureConfig {
    /// Optional visual field descriptor (purely informational in current version).
    pub field: Option<SignatureField>,
    /// Signing reason (e.g. `"Aprovado em reunião de câmara"`).
    pub reason: Option<String>,
    /// Signing location (e.g. `"Lisboa"`).
    pub location: Option<String>,
    /// Bytes reserved for the PKCS#7 DER blob.  Default: 8192.
    pub reserved_bytes: Option<usize>,
}

impl SignatureConfig {
    /// Convert to the lower-level [`SignatureOptions`] used by the backend.
    pub fn to_options(&self) -> SignatureOptions {
        SignatureOptions {
            reason: self
                .reason
                .clone()
                .unwrap_or_else(|| "Assinado digitalmente".into()),
            location: self.location.clone().unwrap_or_else(|| "Portugal".into()),
            reserved_bytes: self.reserved_bytes.unwrap_or(8192),
        }
    }
}

/// Prepares a rendered PDF for external PKCS#7 signing.
///
/// `pdf_bytes` must already contain `/ByteRange` and `/Contents` placeholders,
/// as produced by [`DocumentBuilder::render_prepared_for_signing`].
/// Pass the DER-encoded PKCS#7 blob from your signing system to
/// `PreparedPdf::embed_signature` to obtain the final signed PDF.
///
/// # Example
/// ```rust,no_run
/// use normordis_pdf::{DocumentBuilder, SignatureConfig, sign_pdf};
///
/// let config = SignatureConfig {
///     reason:   Some("Aprovado".into()),
///     location: Some("Lisboa".into()),
///     ..Default::default()
/// };
/// let prepared = DocumentBuilder::new("Acta")
///     .render_prepared_for_signing(config.to_options())?;
///
/// // Sign externally — here we use an empty placeholder:
/// let pkcs7_der: Vec<u8> = vec![]; // replace with real DER from your HSM
/// let signed = sign_pdf(prepared, &config, &pkcs7_der)?;
/// # Ok::<(), normordis_pdf::NormaxisPdfError>(())
/// ```
pub fn sign_pdf(
    prepared: PreparedPdf,
    _config: &SignatureConfig,
    pkcs7_der: &[u8],
) -> Result<Vec<u8>> {
    prepared.embed_signature(pkcs7_der)
}

// ── Internal helper ───────────────────────────────────────────────────────────

/// Find the signature placeholders in raw PDF bytes, patch `/ByteRange`
/// in-place and return the [`PreparedPdf`].
///
/// Requires that the backend wrote:
/// - `/Contents <80808080...>` (reserved_bytes × 0x80 as hex)
/// - `/ByteRange [0 1111111111 1222222222 1333333333]` (36 bytes, patchable)
pub(crate) fn extract_prepared(mut bytes: Vec<u8>, reserved_bytes: usize) -> Result<PreparedPdf> {
    // ── Locate /Contents placeholder ─────────────────────────────────────────
    // Pattern: "/Contents <8080" (first 4 hex chars of the 0x80 run)
    let needle: &[u8] = b"/Contents <8080";
    let search_pos = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .ok_or_else(|| {
            NormaxisPdfError::RenderError(
                "signature preparation failed: /Contents placeholder not found".into(),
            )
        })?;

    // offset of '<' = search_pos + len("/Contents ")
    let contents_start = search_pos + b"/Contents ".len();

    // Verify expected closing '>'
    let close_pos = contents_start + 1 + reserved_bytes * 2;
    if close_pos >= bytes.len() || bytes[close_pos] != b'>' {
        return Err(NormaxisPdfError::RenderError(
            "signature preparation failed: /Contents closing '>' not at expected position".into(),
        ));
    }

    // ── Compute ByteRange ─────────────────────────────────────────────────────
    let a = contents_start as u64; // range1_length (bytes before '<')
    let b = a + 1 + reserved_bytes as u64 * 2 + 1; // range2_start (after '>')
    let range2_len = bytes.len() as u64 - b;

    // ── Locate and patch /ByteRange placeholder ───────────────────────────────
    // Placeholder: "[0 1111111111 1222222222 1333333333]" = exactly 36 bytes
    let br_pattern: &[u8] = b"[0 1111111111 1222222222 1333333333]";
    let br_pos = bytes
        .windows(br_pattern.len())
        .position(|w| w == br_pattern)
        .ok_or_else(|| {
            NormaxisPdfError::RenderError(
                "signature preparation failed: /ByteRange placeholder not found".into(),
            )
        })?;

    // Format: "[0 {:<10} {:<10} {:<10}]" — always 36 bytes
    let patch = format!("[0 {:<10} {:<10} {:<10}]", a, b, range2_len);
    debug_assert_eq!(
        patch.len(),
        br_pattern.len(),
        "ByteRange patch length mismatch"
    );
    bytes[br_pos..br_pos + br_pattern.len()].copy_from_slice(patch.as_bytes());

    Ok(PreparedPdf {
        bytes,
        contents_start,
        reserved_bytes,
    })
}
