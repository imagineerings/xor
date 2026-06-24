pub mod qr_code;
mod tunnel_manager;

pub use tunnel_manager::{TunnelInfo, TunnelManager, TunnelStatus};

use anyhow::Context as _;
use parking_lot::Mutex;
use std::sync::Arc;

/// Cached state for rendering the QR code in the settings UI.
pub struct GlobalTunnelState {
    pub manager: TunnelManager,
    pub cached_connection_string: String,
    pub cached_qr_render_image: Option<Arc<gpui::RenderImage>>,
}

/// Global wrapper for the tunnel manager and its cached QR code image.
/// Registered once at app startup, initialized lazily when the tunnel starts.
pub struct GlobalTunnelManager(pub Mutex<Option<GlobalTunnelState>>);

impl gpui::Global for GlobalTunnelManager {}

/// Decode raw QR code PNG bytes into a GPUI RenderImage (BGRA format).
pub fn render_image_from_png(png_bytes: &[u8]) -> anyhow::Result<Arc<gpui::RenderImage>> {
    let mut data = image::load_from_memory(png_bytes)
        .context("failed to decode QR code PNG")?
        .into_rgba8();
    // GPUI expects BGRA format internally
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let frame = image::Frame::new(data);
    Ok(Arc::new(gpui::RenderImage::new([frame])))
}
