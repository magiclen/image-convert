use std::cmp;
use std::fmt::Debug;

use crate::magick_rust::MagickWand;

// The general config of an image format.
pub trait ImageConfig: Debug {
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
    fn get_sharpen(&self) -> f64;
    fn is_shrink_only(&self) -> bool;
}

// Compute an appropriate sharpen value for the resized image.
pub(crate) fn compute_output_size_sharpen(mw: &MagickWand, config: &ImageConfig) -> (u16, u16, f64) {
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

    let ratio = original_width as f64 / original_height as f64;

    let wr = original_width as f64 / width as f64;
    let hr = original_height as f64 / height as f64;

    if wr >= hr {
        height = (width as f64 / ratio) as u16;
    } else {
        width = (height as f64 * ratio) as u16;
    }

    let mut adjusted_sharpen = config.get_sharpen();

    if adjusted_sharpen < 0f64 {
        let origin_pixels = original_width as u32 * original_height as u32;
        let resize_pixels = width as u32 * height as u32;
        let resize_level = (resize_pixels as f64 / 5000000f64).sqrt();

        let m = cmp::max(origin_pixels, resize_pixels) as f64;
        let n = cmp::min(origin_pixels, resize_pixels) as f64;

        adjusted_sharpen = (resize_level * ((m - n) / m)).min(3f64).max(0.1f64);
    }

    (width, height, adjusted_sharpen)
}

// Compute the output size if it is different.
pub(crate) fn compute_output_size_if_different(mw: &MagickWand, config: &ImageConfig) -> Option<(u16, u16)> {
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

    let ratio = original_width as f64 / original_height as f64;

    let wr = original_width as f64 / width as f64;
    let hr = original_height as f64 / height as f64;

    if wr >= hr {
        height = (width as f64 / ratio) as u16;
    } else {
        width = (height as f64 * ratio) as u16;
    }

    Some((width, height))
}