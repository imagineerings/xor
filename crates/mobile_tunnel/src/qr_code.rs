use anyhow::{Context, Result};

/// Generate a QR code PNG from a connection string.
///
/// The connection string has the format:
/// `baymax-tunnel://{host}:{port}?token={token}`
///
/// Returns the raw PNG bytes of the QR code image.
pub fn generate_qr_code_png(connection_string: &str) -> Result<Vec<u8>> {
    let code = qrcode::QrCode::new(connection_string).context("failed to create QR code")?;
    let image = code
        .render::<image::Luma<u8>>()
        .min_dimensions(200, 200)
        .build();
    let mut buf = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buf, image::ImageFormat::Png)
        .context("failed to encode QR code as PNG")?;
    Ok(buf.into_inner())
}

/// Build a connection string for the QR code payload.
pub fn build_connection_string(host: &str, port: u16, token: Option<&str>) -> String {
    match token {
        Some(token) => format!("baymax-tunnel://{host}:{port}?token={token}"),
        None => format!("baymax-tunnel://{host}:{port}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_connection_string_with_token() {
        let result = build_connection_string("127.0.0.1", 9999, Some("abc123"));
        assert_eq!(result, "baymax-tunnel://127.0.0.1:9999?token=abc123");
    }

    #[test]
    fn test_build_connection_string_without_token() {
        let result = build_connection_string("127.0.0.1", 9999, None);
        assert_eq!(result, "baymax-tunnel://127.0.0.1:9999");
    }

    #[test]
    fn test_generate_qr_code_png_valid() {
        let data = "baymax-tunnel://127.0.0.1:9999?token=test123";
        let png_bytes = generate_qr_code_png(data).expect("should generate QR code");
        // Should be a valid PNG starting with the PNG signature
        assert!(png_bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        // Should be non-trivial in size
        assert!(png_bytes.len() > 200);
    }

    #[test]
    fn test_generate_qr_code_png_empty_string() {
        // Edge case: empty connection string
        let png_bytes = generate_qr_code_png("").expect("should generate QR code for empty string");
        assert!(png_bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
    }

    #[test]
    fn test_generate_qr_code_png_long_string() {
        // Edge case: long connection string with max-length-ish content
        let long_token = "a".repeat(64);
        let data = format!(
            "baymax-tunnel://very-long-hostname.example.com:30000?token={}",
            long_token
        );
        let png_bytes =
            generate_qr_code_png(&data).expect("should generate QR code for long string");
        assert!(png_bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        assert!(png_bytes.len() > 200);
    }
}
