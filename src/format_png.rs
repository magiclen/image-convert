use magick_rust::{MagickError, ResolutionType};

use crate::{
    Crop, ImageResource, InterlaceType, check_output, fetch_magic_wand,
    functions::resize_and_sharpen, image_config::impl_image_config, write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a PNG image.
pub struct PNGConfig {
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
    /// Pixels per inch.
    pub ppi:                 Option<(f64, f64)>,
}

impl PNGConfig {
    /// Create a `PNGConfig` instance with default values.
    /// ```rust,ignore
    /// PNGConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: true,
    ///     ppi: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> PNGConfig {
        PNGConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            shrink_only:         true,
            sharpen:             -1f64,
            respect_orientation: true,
            ppi:                 None,
        }
    }
}

impl Default for PNGConfig {
    #[inline]
    fn default() -> Self {
        PNGConfig::new()
    }
}

impl_image_config!(PNGConfig);

/// Convert an image to a PNG image.
pub fn to_png(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &PNGConfig,
) -> Result<(), MagickError> {
    check_output(output, &["png"])?;

    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    // ImageMagick's PNG encoder reads the quality of the image info instead of the one of the image, where the tens digit is the zlib level and the ones digit is the filter method
    // `95` means the best zlib level with adaptive filtering; `100` would turn the filters off and make the output bigger
    mw.set_compression_quality(95)?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("PNG")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(ResolutionType::PixelsPerInch)?;
    }

    write_output(output, mw, "PNG")
}
