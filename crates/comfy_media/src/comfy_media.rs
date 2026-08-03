use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

pub mod metadata;
pub mod png;

pub use metadata::*;
pub use png::*;

pub fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, image::ImageError> {
    let mut bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)?;
    Ok(bytes)
}
