use magick_rust::{FilterType, MagickError};

use crate::{
    Crop, ImageResource, InterlaceType, compute_output_size_sharpen, fetch_magic_wand,
    image_config::impl_image_config, write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a GIF image.
pub struct GIFConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The width of the output image. `0` means the original width.
    pub width:               u32,
    /// The height of the output image. `0` means the original height.
    pub height:              u32,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only:         bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen:             f64,
    /// Apply orientation from image metadata if available.
    pub respect_orientation: bool,
}

impl GIFConfig {
    /// Create a `GIFConfig` instance with default values.
    /// ```rust,ignore
    /// GIFConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: false,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> GIFConfig {
        GIFConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            shrink_only:         true,
            sharpen:             -1f64,
            respect_orientation: false,
        }
    }
}

impl Default for GIFConfig {
    #[inline]
    fn default() -> Self {
        GIFConfig::new()
    }
}

impl_image_config!(GIFConfig);

/// Convert an image to a GIF image.
pub fn to_gif(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GIFConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;

        mw.sharpen_image(0f64, sharpen)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    mw.set_image_compression_quality(100)?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("GIF")?;

    write_output(output, mw, &["gif"], "GIF")
}
