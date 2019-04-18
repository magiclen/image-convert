use crate::{START_CALL_ONCE, ColorName, InterlaceType, ImageResource, ImageConfig, compute_output_size_sharpen, magick_rust::{MagickWand, PixelWand, bindings}, starts_ends_with_caseless::EndsWithCaseless};

/// The output config of a RAW image with gray colors.
pub struct GrayRawConfig {
    /// The width of the output image. `0` means the original width.
    pub width: u16,
    /// The height of the output image. `0` means the original height.
    pub height: u16,
    /// The color is used for fill up the alpha background.
    pub background_color: Option<ColorName>,
}

impl GrayRawConfig {
    /// Create a `JPGConfig` instance with default values.
    /// ```rust,ignore
    /// GrayRawConfig {
    ///     width: 0u16,
    ///     height: 0u16,
    ///     background_color: None,
    /// }
    /// ```
    pub fn new() -> GrayRawConfig {
        GrayRawConfig {
            width: 0u16,
            height: 0u16,
            background_color: None,
        }
    }
}

impl ImageConfig for GrayRawConfig {
    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn get_sharpen(&self) -> f64 {
        0f64
    }

    fn is_shrink_only(&self) -> bool {
        true
    }
}

/// Convert an image to a RAW image with gray colors.
pub fn to_gray_raw(output: &mut ImageResource, input: &ImageResource, config: &GrayRawConfig) -> Result<(), &'static str> {
    START_CALL_ONCE();

    let mut mw = MagickWand::new();

    match input {
        ImageResource::Path(p) => {
            mw.read_image(p.as_str())?;
        }
        ImageResource::Data(ref b) => {
            mw.read_image_blob(b)?;
        }
    }

    if let Some(background_color) = config.background_color {
        let mut pw = PixelWand::new();
        pw.set_color(background_color.as_str())?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(bindings::AlphaChannelOption_RemoveAlphaChannel)?;
    }

    let (width, height, _) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);

    mw.profile_image("*", None)?;

    mw.set_interlace_scheme(InterlaceType::NoInterlace.ordinal() as bindings::InterlaceType)?;

    mw.set_image_depth(8)?;

    mw.set_image_colorspace(bindings::ColorspaceType_GRAYColorspace)?;

    mw.set_image_format("GRAY")?;

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_caseless_ascii(".raw") {
                return Err("The file extension name is not raw.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("GRAY")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}