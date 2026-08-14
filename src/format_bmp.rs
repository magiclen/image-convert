use magick_rust::{FilterType, MagickError, ResolutionType};

use crate::{
    Color, Crop, ImageResource, InterlaceType, compute_output_size_sharpen, fetch_magic_wand,
    functions::handle_background_color, image_config::impl_image_config, write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a BMP image.
pub struct BMPConfig {
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
    /// The color is used for fill up the alpha background.
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
    ///     respect_orientation: false,
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
            respect_orientation: false,
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
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if let Some(background_color) = config.background_color.as_ref() {
        handle_background_color(&mut mw, background_color)?;
    }

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

    mw.set_image_format("BMP")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(ResolutionType::PixelsPerInch)?;
    }

    write_output(output, mw, &["bmp"], "BMP")
}
