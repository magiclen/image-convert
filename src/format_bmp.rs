use magick_rust::{MagickError, ResolutionType};

use crate::{
    Color, Crop, ImageResource, InterlaceType, check_output, fetch_magic_wand,
    functions::{handle_background_color, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a BMP image.
pub struct BMPConfig {
    /// Remove the metadata stored in the input image.
    /// Images with an ICC profile are converted to sRGB before the profile is removed.
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
    /// The color is used to fill up the alpha background.
    pub background_color:    Option<Color>,
    /// Pixels per inch.
    pub ppi:                 Option<(f64, f64)>,
}

impl BMPConfig {
    /// Create a `BMPConfig` instance with default values.
    /// ```rust,ignore
    /// BMPConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: true,
    ///     background_color: None,
    ///     ppi: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> BMPConfig {
        BMPConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            shrink_only:         true,
            sharpen:             -1f64,
            respect_orientation: true,
            background_color:    None,
            ppi:                 None,
        }
    }
}

impl Default for BMPConfig {
    #[inline]
    fn default() -> Self {
        BMPConfig::new()
    }
}

impl_image_config!(BMPConfig);

/// Convert an image to a BMP image.
pub fn to_bmp(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &BMPConfig,
) -> Result<(), MagickError> {
    check_output(output, &["bmp"])?;

    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if let Some(background_color) = config.background_color.as_ref() {
        handle_background_color(&mut mw, background_color)?;
    }

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("BMP")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(ResolutionType::PixelsPerInch)?;
    }

    write_output(output, mw, "BMP")
}
