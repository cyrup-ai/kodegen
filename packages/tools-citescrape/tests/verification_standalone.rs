// Standalone verification that the core logic compiles correctly
// This file can be checked independently of the main crate

use anyhow::Result;

const ALLOWED_IMAGE_TYPES: &[&str] = &[
    "image/jpeg",
    "image/jpg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    "image/bmp",
    "image/x-icon",
    "image/vnd.microsoft.icon",
];

#[must_use]
pub fn detect_image_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }

    match bytes {
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [0x89, 0x50, 0x4E, 0x47, ..] => Some("image/png"),
        [0x47, 0x49, 0x46, 0x38, ..] => Some("image/gif"),
        [0x52, 0x49, 0x46, 0x46, ..] if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" => {
            Some("image/webp")
        }
        [0x42, 0x4D, ..] => Some("image/bmp"),
        [0x00, 0x00, 0x01, 0x00, ..] => Some("image/x-icon"),
        _ if bytes.starts_with(b"<svg") || bytes.starts_with(b"<?xml") => Some("image/svg+xml"),
        _ => None,
    }
}

pub fn validate_image_content_type(
    claimed_type: Option<&str>,
    bytes: &[u8],
) -> Result<&'static str> {
    let detected_type =
        detect_image_type(bytes).ok_or_else(|| anyhow::anyhow!("Unrecognized image format"))?;

    if !ALLOWED_IMAGE_TYPES.contains(&detected_type) {
        return Err(anyhow::anyhow!(
            "Detected image type not allowed: {detected_type}"
        ));
    }

    if let Some(claimed) = claimed_type {
        let claimed_normalized = claimed.split(';').next().unwrap_or(claimed).trim();

        if !ALLOWED_IMAGE_TYPES.contains(&claimed_normalized) {
            return Err(anyhow::anyhow!(
                "Claimed content type not allowed: {claimed_normalized}"
            ));
        }

        let types_match = detected_type == claimed_normalized
            || (detected_type == "image/jpeg" && claimed_normalized == "image/jpg")
            || (detected_type == "image/jpg" && claimed_normalized == "image/jpeg");

        if !types_match {
            return Err(anyhow::anyhow!(
                "Content-Type mismatch: claimed '{claimed_normalized}' but detected '{detected_type}'"
            ));
        }
    }

    Ok(detected_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jpeg_detection() {
        let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(detect_image_type(&bytes), Some("image/jpeg"));
    }

    #[test]
    fn test_malicious_html_rejected() {
        let html = b"<html><script>alert('XSS')</script></html>";
        let result = validate_image_content_type(Some("text/html"), html);
        assert!(result.is_err());
    }

    #[test]
    fn test_mismatch_rejected() {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0];
        let result = validate_image_content_type(Some("image/png"), &jpeg);
        assert!(result.is_err());
    }
}
