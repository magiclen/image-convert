use crate::{
    compute_output_size_sharpen, fetch_magic_wand, magick_rust::bindings,
    starts_ends_with_caseless::EndsWithCaseless, ImageConfig, ImageResource, InterlaceType,
};

#[derive(Debug)]
/// The output config of a PNG image.
pub struct PNGConfig {
    /// Remain the profile stored in the input image.
    pub remain_profile: bool,
    /// The width of the output image. `0` means the original width.
    pub width: u16,
    /// The height of the output image. `0` means the original height.
    pub height: u16,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only: bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
    /// Pixels per inch.
    pub ppi: f64,
}

impl PNGConfig {
    /// Create a `PNGConfig` instance with default values.
    /// ```rust,ignore
    /// PNGConfig {
    ///     width: 0u16,
    ///     height: 0u16,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     ppi: 72f64,
    /// }
    /// ```
    #[inline]
    pub fn new() -> PNGConfig {
        PNGConfig {
            remain_profile: false,
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
            ppi: 72f64,
        }
    }
}

impl Default for PNGConfig {
    #[inline]
    fn default() -> Self {
        PNGConfig::new()
    }
}

impl ImageConfig for PNGConfig {
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
    fn get_sharpen(&self) -> f64 {
        self.sharpen
    }

    #[inline]
    fn is_shrink_only(&self) -> bool {
        self.shrink_only
    }
}

/// Convert an image to a PNG image.
pub fn to_png(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &PNGConfig,
) -> Result<(), &'static str> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);

        mw.sharpen_image(0f64, sharpen)?;
    }

    if !config.remain_profile {
        mw.profile_image("*", None)?;
    }

    mw.set_image_compression_quality(100)?;

    mw.set_interlace_scheme(InterlaceType::LineInterlace.ordinal() as bindings::InterlaceType)?;

    mw.set_image_format("PNG")?;

    if config.ppi >= 0f64 {
        mw.set_image_resolution(config.ppi, config.ppi)?;
        mw.set_image_units(bindings::ResolutionType_PixelsPerInchResolution)?;
    }

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_caseless_ascii(".png") {
                return Err("The file extension name is not png.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(b) => {
            let mut temp = mw.write_image_blob("PNG")?;
            b.append(&mut temp);
        }
        ImageResource::MagickWand(mw_2) => {
            *mw_2 = mw;
        }
    }

    Ok(())
}
