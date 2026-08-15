use magick_rust::{MagickError, ResolutionType};

use crate::{
    Color, Crop, ImageResource, InterlaceType, fetch_magic_wand,
    functions::{handle_background_color, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a JPEG image.
pub struct JPGConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:            bool,
    /// The width of the output image. `0` means the original width.
    pub width:                     u32,
    /// The height of the output image. `0` means the original height.
    pub height:                    u32,
    /// Crop the image.
    pub crop:                      Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only:               bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen:                   f64,
    /// Apply orientation from image metadata if available.
    pub respect_orientation:       bool,
    /// Use 4:2:0 (chroma quartered) subsampling to reduce the file size.
    pub force_to_chroma_quartered: bool,
    /// From 1 to 100, the higher the better.
    pub quality:                   u8,
    /// The color is used for fill up the alpha background.
    pub background_color:          Option<Color>,
    /// Pixels per inch.
    pub ppi:                       Option<(f64, f64)>,
}

impl JPGConfig {
    /// Create a `JPGConfig` instance with default values.
    /// ```rust,ignore
    /// JPGConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: false,
    ///     force_to_chroma_quartered: true,
    ///     quality: 85u8,
    ///     background_color: None,
    ///     ppi: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> JPGConfig {
        JPGConfig {
            strip_metadata:            true,
            width:                     0u32,
            height:                    0u32,
            crop:                      None,
            shrink_only:               true,
            sharpen:                   -1f64,
            respect_orientation:       false,
            force_to_chroma_quartered: true,
            quality:                   85u8,
            background_color:          None,
            ppi:                       None,
        }
    }
}

impl Default for JPGConfig {
    #[inline]
    fn default() -> Self {
        JPGConfig::new()
    }
}

impl_image_config!(JPGConfig);

/// Convert an image to a JPEG image.
pub fn to_jpg(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &JPGConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if let Some(background_color) = config.background_color.as_ref() {
        handle_background_color(&mut mw, background_color)?;
    }

    if !vector {
        resize_and_sharpen(&mw, config)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    if config.force_to_chroma_quartered {
        mw.set_sampling_factors(&[2f64, 1f64, 1f64])?;
    }

    mw.set_image_compression_quality(config.quality.clamp(1, 100) as usize)?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("JPEG")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(ResolutionType::PixelsPerInch)?;
    }

    write_output(output, mw, &["jpg", "jpeg"], "JPEG")
}
