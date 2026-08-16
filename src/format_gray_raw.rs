use magick_rust::{ColorspaceType, MagickError};

use crate::{
    Color, Crop, ImageConfig, ImageResource, InterlaceType, fetch_magic_wand,
    functions::{handle_background_color, resize_and_sharpen},
    write_output,
};

#[derive(Debug, Clone, PartialEq)]
/// The output config of a RAW image with gray colors.
pub struct GrayRawConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The width of the output image. `0` means the original width.
    pub width:               u32,
    /// The height of the output image. `0` means the original height.
    pub height:              u32,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// Apply orientation from image metadata if available. It is applied anyway when `strip_metadata` is `true`, because removing the metadata would otherwise throw the orientation away.
    pub respect_orientation: bool,
    /// The color is used for fill up the alpha background.
    pub background_color:    Option<Color>,
}

impl GrayRawConfig {
    /// Create a `GrayRawConfig` instance with default values.
    /// ```rust,ignore
    /// GrayRawConfig {
    ///     strip_metadata: true,
    ///     width: 0u32,
    ///     height: 0u32,
    ///     crop: None,
    ///     respect_orientation: true,
    ///     background_color: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> GrayRawConfig {
        GrayRawConfig {
            strip_metadata:      true,
            width:               0u32,
            height:              0u32,
            crop:                None,
            respect_orientation: true,
            background_color:    None,
        }
    }
}

impl Default for GrayRawConfig {
    #[inline]
    fn default() -> Self {
        GrayRawConfig::new()
    }
}

// This config has no `sharpen` and no `shrink_only`, so it cannot use the `impl_image_config` macro.
impl ImageConfig for GrayRawConfig {
    #[inline]
    fn strip_metadata(&self) -> bool {
        self.strip_metadata
    }

    #[inline]
    fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    fn crop(&self) -> Option<Crop> {
        self.crop
    }

    #[inline]
    fn sharpen(&self) -> f64 {
        0f64
    }

    #[inline]
    fn shrink_only(&self) -> bool {
        true
    }

    #[inline]
    fn respect_orientation(&self) -> bool {
        self.respect_orientation
    }
}

/// Convert an image to a RAW image with gray colors.
pub fn to_gray_raw(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GrayRawConfig,
) -> Result<(), MagickError> {
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

    mw.set_interlace_scheme(InterlaceType::No)?;

    mw.set_image_depth(8)?;

    mw.set_image_colorspace(ColorspaceType::GRAY)?;

    mw.set_image_format("GRAY")?;

    write_output(output, mw, &["raw"], "GRAY")
}
