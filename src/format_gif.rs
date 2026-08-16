use magick_rust::MagickError;

use crate::{
    Crop, ImageResource, InterlaceType, fetch_magic_wand,
    functions::{for_each_frame, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
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
    /// Apply orientation from image metadata if available. It is applied anyway when `strip_metadata` is `true`, because removing the metadata would otherwise throw the orientation away.
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
    ///     respect_orientation: true,
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
            respect_orientation: true,
        }
    }
}

impl Default for GIFConfig {
    #[inline]
    fn default() -> Self {
        GIFConfig::new()
    }
}

// GIF stores an animation, so every frame of a multi-frame input image is kept.
impl_image_config!(GIFConfig, true);

/// Convert an image to a GIF image.
///
/// An animated input image keeps its frames, but the output is not layer-optimized, so it can be considerably bigger than the input.
pub fn to_gif(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GIFConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    for_each_frame(&mut mw, |frame| {
        if config.strip_metadata {
            frame.strip_image()?;
        }

        frame.set_image_format("GIF")?;

        Ok(())
    })?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    write_output(output, mw, &["gif"], "GIF")
}
