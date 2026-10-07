use magick_rust::{MagickError, ResolutionType};

use crate::{
    Color, Crop, ImageResource, InterlaceType, check_output, fetch_magic_wand,
    functions::{handle_background_color, resize_and_sharpen},
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a JPEG image.
pub struct JPGConfig {
    /// Remove the metadata stored in the input image.
    /// Images with an ICC profile are converted to sRGB before the profile is removed.
    pub strip_metadata:            bool,
    /// The maximum width of the output image, keeping its aspect ratio.
    /// `0` means no width limit; if both limits are `0`, the image is not resized.
    pub width:                     u32,
    /// The maximum height of the output image, keeping its aspect ratio.
    /// `0` means no height limit; if both limits are `0`, the image is not resized.
    pub height:                    u32,
    /// Crop the image.
    pub crop:                      Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only:               bool,
    /// The sharpening strength; `0` disables sharpening, a negative value uses auto adjustment, and a positive value is used as given.
    /// Vector images rendered at the output size skip resizing and sharpening, even for positive values.
    /// If a vector image needs raster resizing, this setting applies as usual.
    pub sharpen:                   f64,
    /// Apply orientation from image metadata if available. It is applied anyway when `strip_metadata` is `true`, because removing the metadata would otherwise throw the orientation away.
    pub respect_orientation:       bool,
    /// Use 4:2:0 (chroma quartered) subsampling to reduce the file size.
    pub force_to_chroma_quartered: bool,
    /// From 1 to 100, the higher the better. `None` means to keep the quality of the input image, which is the quality **ImageMagick** estimates from the quantization tables of an input JPEG image, or its own default when the input image is not a JPEG image.
    pub quality:                   Option<u8>,
    /// The color is used to fill up the alpha background.
    /// If it is `None`, transparent pixels show the colors hidden under them, which are usually black, because this format has no alpha channel.
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
    ///     respect_orientation: true,
    ///     force_to_chroma_quartered: true,
    ///     quality: Some(85u8),
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
            respect_orientation:       true,
            force_to_chroma_quartered: true,
            quality:                   Some(85u8),
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
    check_output(output, &["jpg", "jpeg"])?;

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

    if config.force_to_chroma_quartered {
        // This API writes JPEG ratio notation, so these values mean 4:2:0.
        mw.set_sampling_factors(&[4f64, 2f64, 0f64])?;
    }

    // the encoder falls back to the quality of the image itself, which is the one ImageMagick estimated when it read an input JPEG image
    if let Some(quality) = config.quality {
        mw.set_image_compression_quality(quality.clamp(1, 100) as usize)?;
    }

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("JPEG")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(ResolutionType::PixelsPerInch)?;
    }

    write_output(output, mw, "JPEG")
}
