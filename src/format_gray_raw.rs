use magick_rust::{AlphaChannelOption, ColorspaceType, FilterType, MagickError, PixelWand};
use str_utils::EndsWithIgnoreAsciiCase;

use crate::{
    compute_output_size_sharpen, fetch_magic_wand, ColorName, Crop, ImageConfig, ImageResource,
    InterlaceType,
};

#[derive(Debug)]
/// The output config of a RAW image with gray colors.
pub struct GrayRawConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:   bool,
    /// The width of the output image. `0` means the original width.
    pub width:            u16,
    /// The height of the output image. `0` means the original height.
    pub height:           u16,
    /// Crop the image.
    pub crop:             Option<Crop>,
    /// The color is used for fill up the alpha background.
    pub background_color: Option<ColorName>,
}

impl GrayRawConfig {
    /// Create a `GrayRawConfig` instance with default values.
    /// ```rust,ignore
    /// GrayRawConfig {
    ///     strip_metadata: true,
    ///     width: 0u16,
    ///     height: 0u16,
    ///     crop: None,
    ///     background_color: None,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> GrayRawConfig {
        GrayRawConfig {
            strip_metadata:   true,
            width:            0u16,
            height:           0u16,
            crop:             None,
            background_color: None,
        }
    }
}

impl Default for GrayRawConfig {
    #[inline]
    fn default() -> Self {
        GrayRawConfig::new()
    }
}

impl ImageConfig for GrayRawConfig {
    #[inline]
    fn is_strip_metadata(&self) -> bool {
        self.strip_metadata
    }

    #[inline]
    fn get_width(&self) -> u16 {
        self.width
    }

    #[inline]
    fn get_height(&self) -> u16 {
        self.height
    }

    #[inline]
    fn get_crop(&self) -> Option<Crop> {
        self.crop
    }

    #[inline]
    fn get_sharpen(&self) -> f64 {
        0f64
    }

    #[inline]
    fn is_shrink_only(&self) -> bool {
        true
    }
}

/// Convert an image to a RAW image with gray colors.
pub fn to_gray_raw(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GrayRawConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if let Some(background_color) = config.background_color {
        let mut pw = PixelWand::new();
        pw.set_color(background_color.as_str())?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(AlphaChannelOption::Remove)?;
    }

    if !vector {
        let (width, height, _) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    mw.set_interlace_scheme(InterlaceType::No)?;

    mw.set_image_depth(8)?;

    mw.set_image_colorspace(ColorspaceType::GRAY)?;

    mw.set_image_format("GRAY")?;

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_ignore_ascii_case_with_lowercase(".raw") {
                return Err("The file extension name is not raw.".into());
            }

            mw.write_image(p.as_str())?;
        },
        ImageResource::Data(b) => {
            let mut temp = mw.write_image_blob("GRAY")?;
            b.append(&mut temp);
        },
        ImageResource::MagickWand(mw_2) => {
            *mw_2 = mw;
        },
    }

    Ok(())
}
