use std::fmt::Debug;

use magick_rust::MagickWand;

use crate::Crop;

/// The general config of an image format.
pub trait ImageConfig: Debug {
    /// Whether to remove the metadata stored in the input image.
    fn strip_metadata(&self) -> bool;
    /// The maximum width of the output image, keeping its aspect ratio.
    /// `0` means no width limit; if both limits are `0`, the image is not resized.
    fn width(&self) -> u32;
    /// The maximum height of the output image, keeping its aspect ratio.
    /// `0` means no height limit; if both limits are `0`, the image is not resized.
    fn height(&self) -> u32;
    /// How to crop the input image.
    fn crop(&self) -> Option<Crop>;
    /// The sharpening strength; `0` disables sharpening, a negative value uses auto adjustment, and a positive value is used as given.
    /// Vector images rendered at the output size skip resizing and sharpening, even for positive values.
    /// If a vector image needs raster resizing, this setting applies as usual.
    fn sharpen(&self) -> f64;
    /// Whether to shrink the image only, not to enlarge it.
    fn shrink_only(&self) -> bool;
    /// Whether to apply the orientation stored in the image metadata.
    ///
    /// The orientation is applied anyway when `strip_metadata` is `true`, because removing the metadata would otherwise throw the orientation away and leave the image lying on its side.
    fn respect_orientation(&self) -> bool;
    /// Whether the output format can store every frame of a multi-frame input image, such as an animated GIF. `false` means to keep the first frame only.
    fn keep_frames(&self) -> bool {
        false
    }
}

/// Implement `ImageConfig` for a config struct which has a field for every method of the trait.
macro_rules! impl_image_config {
    ($name:ident) => {
        impl_image_config!($name, false);
    };
    ($name:ident, $keep_frames:expr) => {
        impl $crate::ImageConfig for $name {
            #[inline]
            fn strip_metadata(&self) -> bool {
                self.strip_metadata
            }

            #[inline]
            fn width(&self) -> u32 {
                self.width
            }

            #[inline]
            fn height(&self) -> u32 {
                self.height
            }

            #[inline]
            fn crop(&self) -> Option<$crate::Crop> {
                self.crop
            }

            #[inline]
            fn sharpen(&self) -> f64 {
                self.sharpen
            }

            #[inline]
            fn shrink_only(&self) -> bool {
                self.shrink_only
            }

            #[inline]
            fn respect_orientation(&self) -> bool {
                self.respect_orientation
            }

            #[inline]
            fn keep_frames(&self) -> bool {
                $keep_frames
            }
        }
    };
}

pub(crate) use impl_image_config;

// Compute an appropriate sharpen value for the resized image.
pub(crate) fn compute_output_size_sharpen(
    mw: &MagickWand,
    config: &impl ImageConfig,
) -> (u32, u32, f64) {
    let original_width = mw.get_image_width() as u32;
    let original_height = mw.get_image_height() as u32;

    let (width, height) = compute_output_size(
        config.shrink_only(),
        original_width,
        original_height,
        config.width(),
        config.height(),
    )
    .unwrap_or((original_width, original_height));

    let mut adjusted_sharpen = config.sharpen();

    if adjusted_sharpen < 0f64 {
        let origin_pixels = f64::from(original_width) * f64::from(original_height);
        let resize_pixels = f64::from(width) * f64::from(height);
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
) -> Option<(u32, u32)> {
    compute_output_size(
        config.shrink_only(),
        mw.get_image_width() as u32,
        mw.get_image_height() as u32,
        config.width(),
        config.height(),
    )
}

/// Compute the output size while keeping the aspect ratio of the input size.
///
/// `max_width` and `max_height` are the limits of the output size. `0` means no limit. If both of them are `0`, the output size is the same as the input size.
///
/// If it returns `None`, the size remains the same.
pub fn compute_output_size(
    shrink_only: bool,
    input_width: u32,
    input_height: u32,
    max_width: u32,
    max_height: u32,
) -> Option<(u32, u32)> {
    if input_width == 0 || input_height == 0 {
        return None;
    }

    let mut width = max_width;
    let mut height = max_height;

    if shrink_only {
        if width == 0 || width > input_width {
            width = input_width;
        }

        if height == 0 || height > input_height {
            height = input_height;
        }
    }

    if width == 0 && height == 0 {
        return None;
    }

    let input_width_f64 = f64::from(input_width);
    let input_height_f64 = f64::from(input_height);

    let ratio = input_width_f64 / input_height_f64;

    let (width, height) = if width == 0 {
        ((f64::from(height) * ratio).round() as u32, height)
    } else if height == 0 {
        (width, (f64::from(width) / ratio).round() as u32)
    } else {
        // fit the output size into the box formed by `width` and `height`
        let wr = input_width_f64 / f64::from(width);
        let hr = input_height_f64 / f64::from(height);

        if wr >= hr {
            (width, (f64::from(width) / ratio).round() as u32)
        } else {
            ((f64::from(height) * ratio).round() as u32, height)
        }
    };

    // rounding may produce a zero when the input is extremely thin
    let width = width.max(1);
    let height = height.max(1);

    if width == input_width && height == input_height { None } else { Some((width, height)) }
}
