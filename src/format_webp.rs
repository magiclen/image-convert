use magick_rust::{MagickError, ResolutionType};

use crate::{
    Crop, ImageResource, InterlaceType, fetch_magic_wand,
    functions::{for_each_frame, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a WEBP image.
pub struct WEBPConfig {
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
    /// From 0 to 100, the higher the better.
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
/// An animated input image keeps its frames if **ImageMagick** was built with the `webpmux` delegate.
pub fn to_webp(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &WEBPConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    let quality = config.quality.min(100) as usize;

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

    write_output(output, mw, &["webp"], "WEBP")
}
