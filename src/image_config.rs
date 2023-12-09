use std::fmt::Debug;

use magick_rust::MagickWand;

use crate::Crop;

// The general config of an image format.
pub trait ImageConfig: Debug {
    fn is_remain_profile(&self) -> bool;
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
    fn get_crop(&self) -> Option<Crop>;
    fn get_sharpen(&self) -> f64;
    fn is_shrink_only(&self) -> bool;
}

// Compute an appropriate sharpen value for the resized image.
pub(crate) fn compute_output_size_sharpen(
    mw: &MagickWand,
    config: &impl ImageConfig,
) -> (u16, u16, f64) {
    let original_width = mw.get_image_width() as u16;
    let original_height = mw.get_image_height() as u16;

    let (width, height) = compute_output_size(
        config.is_shrink_only(),
        original_width,
        original_height,
        config.get_width(),
        config.get_height(),
    )
    .unwrap_or((original_width, original_height));

    let mut adjusted_sharpen = config.get_sharpen();

    if adjusted_sharpen < 0f64 {
        let origin_pixels = original_width as f64 * original_height as f64;
        let resize_pixels = width as f64 * height as f64;
        let resize_level = (resize_pixels / 5_000_000f64).sqrt();

        let m;
        let n = if origin_pixels >= resize_pixels {
            m = origin_pixels;
            resize_pixels
        } else {
            m = resize_pixels;
            origin_pixels
        };

        adjusted_sharpen = (resize_level * ((m - n) / m)).min(3f64);
    }

    (width, height, adjusted_sharpen)
}

#[inline]
pub(crate) fn compute_output_size_if_different(
    mw: &MagickWand,
    config: &impl ImageConfig,
) -> Option<(u16, u16)> {
    compute_output_size(
        config.is_shrink_only(),
        mw.get_image_width() as u16,
        mw.get_image_height() as u16,
        config.get_width(),
        config.get_height(),
    )
}

/// Compute the output size. If it returns `None`, the size remains the same.
pub fn compute_output_size(
    shrink_only: bool,
    input_width: u16,
    input_height: u16,
    max_width: u16,
    max_height: u16,
) -> Option<(u16, u16)> {
    let mut width = max_width;
    let mut height = max_height;

    if shrink_only {
        if width == 0 || width > input_width {
            width = input_width;
        }
        if height == 0 || height > input_height {
            height = input_height;
        }
    } else {
        if width == 0 {
            width = input_width;
        }
        if height == 0 {
            height = input_height;
        }
    }

    if width == input_width && height == input_height {
        return None;
    }

    let input_width_f64 = f64::from(input_width);
    let input_height_f64 = f64::from(input_height);
    let width_f64 = f64::from(width);
    let height_f64 = f64::from(height);

    let ratio = input_width_f64 / input_height_f64;

    let wr = input_width_f64 / width_f64;
    let hr = input_height_f64 / height_f64;

    if wr >= hr {
        height = (width_f64 / ratio).round() as u16;
    } else {
        width = (height_f64 * ratio).round() as u16;
    }

    Some((width, height))
}
