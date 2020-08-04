use crate::{
    compute_output_size_sharpen, fetch_magic_wand,
    magick_rust::{bindings, PixelWand},
    str_utils::EndsWithIgnoreAsciiCaseMultiple,
    ColorName, Crop, ImageConfig, ImageResource, InterlaceType,
};

#[derive(Debug)]
/// The output config of a JPEG image.
pub struct JPGConfig {
    /// Remain the profile stored in the input image.
    pub remain_profile: bool,
    /// The width of the output image. `0` means the original width.
    pub width: u16,
    /// The height of the output image. `0` means the original height.
    pub height: u16,
    /// Crop the image.
    pub crop: Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only: bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
    /// Use 4:2:0 (chroma quartered) subsampling to reduce the file size.
    pub force_to_chroma_quartered: bool,
    /// From 1 to 100, the higher the better.
    pub quality: u8,
    /// The color is used for fill up the alpha background.
    pub background_color: Option<ColorName>,
    /// Pixels per inch.
    pub ppi: Option<(f64, f64)>,
}

impl JPGConfig {
    /// Create a `JPGConfig` instance with default values.
    /// ```rust,ignore
    /// JPGConfig {
    ///     remain_profile: false,
    ///     width: 0u16,
    ///     height: 0u16,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     force_to_chroma_quartered: true,
    ///     quality: 85u8,
    ///     background_color: None,
    ///     ppi: None,
    /// }
    /// ```
    #[inline]
    pub fn new() -> JPGConfig {
        JPGConfig {
            remain_profile: false,
            width: 0u16,
            height: 0u16,
            crop: None,
            shrink_only: true,
            sharpen: -1f64,
            force_to_chroma_quartered: true,
            quality: 85u8,
            background_color: None,
            ppi: None,
        }
    }
}

impl Default for JPGConfig {
    #[inline]
    fn default() -> Self {
        JPGConfig::new()
    }
}

impl ImageConfig for JPGConfig {
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
        self.sharpen
    }

    #[inline]
    fn is_shrink_only(&self) -> bool {
        self.shrink_only
    }
}

/// Convert an image to a JPEG image.
pub fn to_jpg(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &JPGConfig,
) -> Result<(), &'static str> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if let Some(background_color) = config.background_color {
        let mut pw = PixelWand::new();
        pw.set_color(background_color.as_str())?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(bindings::AlphaChannelOption_RemoveAlphaChannel)?;
    }

    if !vector {
        let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);

        mw.sharpen_image(0f64, sharpen)?;
    }

    if !config.remain_profile {
        mw.profile_image("*", None)?;
    }

    if config.force_to_chroma_quartered {
        mw.set_sampling_factors(&[2f64, 1f64, 1f64])?;
    }

    mw.set_image_compression_quality(config.quality.min(100).max(1) as usize)?;

    mw.set_interlace_scheme(InterlaceType::LineInterlace.ordinal() as bindings::InterlaceType)?;

    mw.set_image_format("JPEG")?;

    if let Some((x, y)) = config.ppi {
        mw.set_image_resolution(x.max(0f64), y.max(0f64))?;
        mw.set_image_units(bindings::ResolutionType_PixelsPerInchResolution)?;
    }

    match output {
        ImageResource::Path(p) => {
            if p.ends_with_ignore_ascii_case_with_lowercase_multiple(&[".jpg", ".jpeg"]).is_none() {
                return Err("The file extension name is not jpg or jpeg.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(b) => {
            let mut temp = mw.write_image_blob("JPEG")?;
            b.append(&mut temp);
        }
        ImageResource::MagickWand(mw_2) => {
            *mw_2 = mw;
        }
    }

    Ok(())
}
