use std::fmt::Debug;

use crate::magick_rust::MagickWand;

// The general config of an image format.
pub trait ImageConfig: Debug {
    fn is_remain_profile(&self) -> bool;
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
    fn get_sharpen(&self) -> f64;
    fn is_shrink_only(&self) -> bool;
}

// Compute an appropriate sharpen value for the resized image.
pub(crate) fn compute_output_size_sharpen(
    mw: &MagickWand,
    config: &dyn ImageConfig,
) -> (u16, u16, f64) {
    let mut width = config.get_width();
    let mut height = config.get_height();
    let original_width = mw.get_image_width() as u16;
    let original_height = mw.get_image_height() as u16;

    if config.is_shrink_only() {
        if width == 0 || width > original_width {
            width = original_width
        }
        if height == 0 || height > original_height {
            height = original_height
        }
    } else {
        if width == 0 {
            width = original_width
        }
        if height == 0 {
            height = original_height
        }
    }

    let original_width_f64 = f64::from(original_width);
    let original_height_f64 = f64::from(original_height);
    let width_f64 = f64::from(width);
    let height_f64 = f64::from(height);

    let ratio = original_width_f64 / original_height_f64;

    let wr = original_width_f64 / width_f64;
    let hr = original_height_f64 / height_f64;

    if wr >= hr {
        height = (width_f64 / ratio).round() as u16;
    } else {
        width = (height_f64 * ratio).round() as u16;
    }

    let mut adjusted_sharpen = config.get_sharpen();

    if adjusted_sharpen < 0f64 {
        let origin_pixels = original_width_f64 * original_height_f64;
        let resize_pixels = width_f64 * height_f64;
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

// Compute the output size if it is different.
pub(crate) fn compute_output_size_if_different(
    mw: &MagickWand,
    config: &dyn ImageConfig,
) -> Option<(u16, u16)> {
    let mut width = config.get_width();
    let mut height = config.get_height();
    let original_width = mw.get_image_width() as u16;
    let original_height = mw.get_image_height() as u16;

    if config.is_shrink_only() {
        if width == 0 || width > original_width {
            width = original_width
        }
        if height == 0 || height > original_height {
            height = original_height
        }
    } else {
        if width == 0 {
            width = original_width
        }
        if height == 0 {
            height = original_height
        }
    }

    if width == original_width && height == original_height {
        return None;
    }

    let original_width_f64 = f64::from(original_width);
    let original_height_f64 = f64::from(original_height);
    let width_f64 = f64::from(width);
    let height_f64 = f64::from(height);

    let ratio = original_width_f64 / original_height_f64;

    let wr = original_width_f64 / width_f64;
    let hr = original_height_f64 / height_f64;

    if wr >= hr {
        height = (width_f64 / ratio).round() as u16;
    } else {
        width = (height_f64 * ratio).round() as u16;
    }

    Some((width, height))
}
