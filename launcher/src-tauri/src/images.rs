//! Image validation for Save icons and the custom background. Images are decoded (which rejects anything that is not
//! a real PNG/JPEG/WebP), size-limited, and re-encoded as PNG into managed storage.

use crate::bail;
use crate::error::AppResult;
use image::{GenericImageView, ImageFormat};
use std::io::Cursor;
use std::path::Path;

const MAX_INPUT_BYTES: u64 = 25 * 1024 * 1024;
const MAX_BACKGROUND_EDGE: u32 = 3840;
pub const ICON_SIZE: u32 = 256;

fn detect(bytes: &[u8]) -> AppResult<ImageFormat> {
    match image::guess_format(bytes) {
        Ok(f @ (ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)) => Ok(f),
        _ => bail!("IMAGE", "Use a PNG, JPEG or WebP image."),
    }
}

fn read_limited(path: &Path) -> AppResult<Vec<u8>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_INPUT_BYTES {
        bail!("IMAGE", "That image is larger than 25 MB.");
    }
    Ok(std::fs::read(path)?)
}

fn encode_png(img: &image::DynamicImage) -> AppResult<Vec<u8>> {
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
        .map_err(|e| crate::error::AppError::new("IMAGE", format!("Could not save the image: {e}")))?;
    Ok(out)
}

/// Background: keep aspect ratio, cap the longest edge, store as PNG (readable by both launcher and game).
pub fn prepare_background(path: &Path) -> AppResult<Vec<u8>> {
    let bytes = read_limited(path)?;
    let format = detect(&bytes)?;
    let img = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| crate::error::AppError::new("IMAGE", "That image could not be read."))?;
    let (w, h) = img.dimensions();
    if w < 320 || h < 180 {
        bail!("IMAGE", "Choose an image at least 320 × 180 pixels.");
    }
    let img = if w.max(h) > MAX_BACKGROUND_EDGE {
        img.resize(MAX_BACKGROUND_EDGE, MAX_BACKGROUND_EDGE, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    encode_png(&img.to_rgb8().into())
}

/// Icon: the launcher crops/fits on a canvas and sends a PNG; it is validated and normalised to 256 × 256 here.
pub fn prepare_icon(png_bytes: &[u8]) -> AppResult<Vec<u8>> {
    if png_bytes.len() as u64 > MAX_INPUT_BYTES {
        bail!("IMAGE", "That image is too large.");
    }
    let format = detect(png_bytes)?;
    let img = image::load_from_memory_with_format(png_bytes, format)
        .map_err(|_| crate::error::AppError::new("IMAGE", "That image could not be read."))?;
    let img = img.resize_to_fill(ICON_SIZE, ICON_SIZE, image::imageops::FilterType::Lanczos3);
    encode_png(&img)
}

/// Validate a file the user picked before the UI shows its crop preview.
pub fn inspect(path: &Path) -> AppResult<(Vec<u8>, &'static str, u32, u32)> {
    let bytes = read_limited(path)?;
    let format = detect(&bytes)?;
    let img = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| crate::error::AppError::new("IMAGE", "That image could not be read."))?;
    let (w, h) = img.dimensions();
    if w < 16 || h < 16 {
        bail!("IMAGE", "That image is too small.");
    }
    let mime = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        _ => "image/webp",
    };
    Ok((bytes, mime, w, h))
}

/// Minecraft avatar: the 8×8 face plus the hat overlay, scaled up crisply.
pub fn avatar_from_skin(skin_png: &[u8]) -> Option<Vec<u8>> {
    let skin = image::load_from_memory_with_format(skin_png, ImageFormat::Png).ok()?.to_rgba8();
    if skin.width() < 64 || skin.height() < 32 {
        return None;
    }
    let mut face = image::RgbaImage::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let base = *skin.get_pixel(8 + x, 8 + y);
            let hat = *skin.get_pixel(40 + x, 8 + y);
            let px = if hat[3] > 0 { hat } else { base };
            face.put_pixel(x, y, image::Rgba([px[0], px[1], px[2], 255]));
        }
    }
    let scaled = image::imageops::resize(&face, 64, 64, image::imageops::FilterType::Nearest);
    encode_png(&image::DynamicImage::ImageRgba8(scaled)).ok()
}
