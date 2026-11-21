use anyhow::{Context, Result};
use qrcode::QrCode;

/// Renders `url` as a QR code, returned as a base64 SVG data URI.
pub fn encode_svg_data_uri(url: &str) -> Result<String> {
    let code = QrCode::new(url.as_bytes()).context("Failed to generate QR code")?;

    let svg = code
        .render::<qrcode::render::svg::Color>()
        .quiet_zone(true)
        .build();

    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(svg.as_bytes());
    Ok(format!("data:image/svg+xml;base64,{}", b64))
}
