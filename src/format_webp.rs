use magick_rust::{MagickError, ResolutionType};

use crate::{
    Crop, ImageResource, InterlaceType, check_output,
    functions::{fetch_magic_wand_for_format, for_each_frame, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a WEBP image.
pub struct WEBPConfig {
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
    /// From 1 to 100, the higher the better. `0` is treated as `1`.
    pub quality:             u8,
    /// Pixels per inch.
    pub ppi:                 Option<(f64, f64)>,
}

impl WEBPConfig {
    /// Create a `WEBPConfig` instance with default values.
    /// ```rust,ignore
    /// WEBPConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: true,
    ///     quality: 85u8,
    ///     ppi: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> WEBPConfig {
        WEBPConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            shrink_only:         true,
            sharpen:             -1f64,
            respect_orientation: true,
            quality:             85u8,
            ppi:                 None,
        }
    }
}

impl Default for WEBPConfig {
    #[inline]
    fn default() -> Self {
        WEBPConfig::new()
    }
}

// WEBP stores an animation, so every frame of a multi-frame input image is kept.
impl_image_config!(WEBPConfig, true);

/// Convert an image to a WEBP image.
///
/// An animated input image keeps its animation. Writing an animation returns an error if **ImageMagick** was built without the `webpmux` delegate.
///
/// The pages of a multi-page document, such as a TIFF document, are not put onto a shared canvas, so pages larger than the first one are cropped.
pub fn to_webp(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &WEBPConfig,
) -> Result<(), MagickError> {
    check_output(output, &["webp"])?;

    let (mut mw, vector) = fetch_magic_wand_for_format(input, config, "WEBP")?;

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    // ImageMagick treats `0` as an undefined quality and falls back to its default one
    let quality = config.quality.clamp(1, 100) as usize;

    for_each_frame(&mut mw, |frame| {
        if config.strip_metadata {
            frame.strip_image()?;
        }

        frame.set_image_compression_quality(quality)?;

        frame.set_image_format("WEBP")?;

        if let Some((x, y)) = config.ppi {
            frame.set_image_resolution(x.max(0f64), y.max(0f64))?;
            frame.set_image_units(ResolutionType::PixelsPerInch)?;
        }

        Ok(())
    })?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    write_output(output, mw, "WEBP")
}
