use std::io::Cursor;

use image::imageops::FilterType;
use image::{DynamicImage, ImageReader, Limits};

use super::{ICON_SIZE, encode_png};

const MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;

/// Converts any supported raster image (ICO, PNG, JPEG, GIF, WebP) to an icon-sized PNG.
pub(super) fn icon_png(bytes: &[u8]) -> Option<Vec<u8>> {
    let pixels = decode(bytes)?
        .resize_exact(ICON_SIZE, ICON_SIZE, FilterType::Lanczos3)
        .to_rgba8();
    encode_png(pixels.as_raw()).ok()
}

fn decode(bytes: &[u8]) -> Option<DynamicImage> {
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader.decode().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageFormat, RgbaImage};

    fn encoded(format: ImageFormat, size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        RgbaImage::from_pixel(size, size, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut Cursor::new(&mut bytes), format)
            .expect("fixture encoding");
        bytes
    }

    fn png_dimensions(png: &[u8]) -> (u32, u32) {
        let reader = png::Decoder::new(Cursor::new(png))
            .read_info()
            .expect("valid PNG");
        (reader.info().width, reader.info().height)
    }

    #[test]
    fn raster_scales_png_down_to_icon_size() {
        let png = icon_png(&encoded(ImageFormat::Png, 128)).expect("icon");
        assert_eq!(png_dimensions(&png), (ICON_SIZE, ICON_SIZE));
    }

    #[test]
    fn raster_scales_small_ico_up_to_icon_size() {
        let png = icon_png(&encoded(ImageFormat::Ico, 16)).expect("icon");
        assert_eq!(png_dimensions(&png), (ICON_SIZE, ICON_SIZE));
    }

    #[test]
    fn raster_converts_gif() {
        assert!(icon_png(&encoded(ImageFormat::Gif, 48)).is_some());
    }

    #[test]
    fn raster_rejects_non_images() {
        assert!(icon_png(b"<html>404 Not Found</html>").is_none());
        assert!(icon_png(b"<svg xmlns='http://www.w3.org/2000/svg'/>").is_none());
        assert!(icon_png(&[]).is_none());
    }
}
