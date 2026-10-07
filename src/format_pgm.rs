use magick_rust::MagickError;

use crate::{
    Color, Crop, ImageResource, InterlaceType, check_output,
    functions::{fetch_magic_wand_for_format, handle_background_color, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a PGM image.
pub struct PGMConfig {
    /// Remove the metadata stored in the input image.
    /// ICC profiles are applied before converting to gray, even when this is `false`.
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
    /// If it is `None`, transparent pixels show the colors hidden under them, which are usually black, because this format has no alpha channel.
    pub background_color:    Option<Color>,
}

impl PGMConfig {
    /// Create a `PGMConfig` instance with default values.
    /// ```rust,ignore
    /// PGMConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     respect_orientation: true,
    ///     background_color: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> PGMConfig {
        PGMConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            shrink_only:         true,
            sharpen:             -1f64,
            respect_orientation: true,
            background_color:    None,
        }
    }
}

impl Default for PGMConfig {
    #[inline]
    fn default() -> Self {
        PGMConfig::new()
    }
}

impl_image_config!(PGMConfig);

/// Convert an image to a PGM image.
pub fn to_pgm(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &PGMConfig,
) -> Result<(), MagickError> {
    check_output(output, &["pgm"])?;

    let (mut mw, vector) = fetch_magic_wand_for_format(input, config, "PGM")?;

    if let Some(background_color) = config.background_color.as_ref() {
        handle_background_color(&mut mw, background_color)?;
    }

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    mw.set_interlace_scheme(InterlaceType::No)?;

    mw.set_image_format("PGM")?;

    write_output(output, mw, "PGM")
}
