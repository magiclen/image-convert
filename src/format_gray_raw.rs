use enum_ordinalize::Ordinalize;
use magick_rust::{bindings, MagickError, PixelWand};
use str_utils::EndsWithIgnoreAsciiCase;

use crate::{
    compute_output_size_sharpen, fetch_magic_wand, ColorName, Crop, ImageConfig, ImageResource,
    InterlaceType,
};

#[derive(Debug)]
/// The output config of a RAW image with gray colors.
pub struct GrayRawConfig {
    /// Remain the profile stored in the input image.
    pub remain_profile:   bool,
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
    ///     remain_profile: false,
    ///     width: 0u16,
    ///     height: 0u16,
    ///     crop: None,
    ///     background_color: None,
    /// }
    /// ```
    #[inline]
    pub fn new() -> GrayRawConfig {
        GrayRawConfig {
            remain_profile:   false,
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
    fn is_remain_profile(&self) -> bool {
        self.remain_profile
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
        mw.set_image_alpha_channel(bindings::AlphaChannelOption_RemoveAlphaChannel)?;
    }

    if !vector {
        let (width, height, _) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);
    }

    if !config.remain_profile {
        mw.profile_image("*", None)?;
    }

    mw.set_interlace_scheme(InterlaceType::NoInterlace.ordinal() as bindings::InterlaceType)?;

    mw.set_image_depth(8)?;

    mw.set_image_colorspace(bindings::ColorspaceType_GRAYColorspace)?;

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
