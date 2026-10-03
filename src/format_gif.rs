use magick_rust::MagickError;

use crate::{
    Crop, ImageResource, InterlaceType, check_output,
    functions::{fetch_magic_wand_for_format, for_each_frame, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a GIF image.
pub struct GIFConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The maximum width of the output image, keeping its aspect ratio.
    /// `0` means no width limit; if both limits are `0`, the image is not resized.
    pub width:               u32,
    /// The maximum height of the output image, keeping its aspect ratio.
    /// `0` means no height limit; if both limits are `0`, the image is not resized.
    pub height:              u32,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only:         bool,
    /// The sharpening strength; `0` disables sharpening, a negative value uses auto adjustment, and a positive value is used as given.
    /// Vector images rendered at the output size skip resizing and sharpening, even for positive values.
    /// If a vector image needs raster resizing, this setting applies as usual.
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
/// An animated input image keeps its frames. An unedited GIF keeps its layer optimization; frames which need compositing are not optimized again, so the output can be bigger.
///
/// The pages of a multi-page document, such as a TIFF document, are not put onto a shared canvas, so pages larger than the first one exceed the logical screen of the GIF and may be clipped by viewers.
pub fn to_gif(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GIFConfig,
) -> Result<(), MagickError> {
    check_output(output, &["gif"])?;

    let (mut mw, vector) = fetch_magic_wand_for_format(input, config, "GIF")?;

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

    write_output(output, mw, "GIF")
}
