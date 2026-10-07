use magick_rust::{CompressionType, MagickError, ResolutionType};

use crate::{
    Color, Crop, ImageResource, InterlaceType, check_output,
    functions::{
        fetch_magic_wand_for_format, for_each_frame, handle_background_color, resize_and_sharpen,
    },
    image_config::impl_image_config,
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a TIFF image.
pub struct TIFFConfig {
    /// Remove the metadata stored in the input image.
    /// Images with an ICC profile are converted to sRGB before the profile is removed.
    /// Source page labels are also removed.
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

impl TIFFConfig {
    /// Create a `TIFFConfig` instance with default values.
    /// ```rust,ignore
    /// TIFFConfig {
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
    pub const fn new() -> TIFFConfig {
        TIFFConfig {
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

impl Default for TIFFConfig {
    #[inline]
    fn default() -> Self {
        TIFFConfig::new()
    }
}

// TIFF stores a multi-page document, so every frame of a multi-frame input image is kept.
impl_image_config!(TIFFConfig, true);

/// Convert an image to a TIFF image.
///
/// A multi-page input image keeps its pages, and each of them is resized on its own. The output is compressed with LZW, which is lossless.
pub fn to_tiff(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &TIFFConfig,
) -> Result<(), MagickError> {
    check_output(output, &["tif", "tiff"])?;

    let (mut mw, vector) = fetch_magic_wand_for_format(input, config, "TIFF")?;

    if let Some(background_color) = config.background_color.as_ref() {
        handle_background_color(&mut mw, background_color)?;
    }

    if !vector {
        resize_and_sharpen(&mut mw, config)?;
    }

    for_each_frame(&mut mw, |frame| {
        if config.strip_metadata {
            frame.strip_image()?;
            frame.set_image_property("label", "")?;
        }

        frame.set_image_format("TIFF")?;

        if let Some((x, y)) = config.ppi {
            frame.set_image_resolution(x.max(0f64), y.max(0f64))?;
            frame.set_image_units(ResolutionType::PixelsPerInch)?;
        }

        Ok(())
    })?;

    // ImageMagick's TIFF encoder reads the compression of the image info instead of the one of the image
    // LZW is lossless, and the encoder enables the horizontal predictor for it.
    mw.set_compression(CompressionType::LZW)?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    write_output(output, mw, "TIFF")
}
