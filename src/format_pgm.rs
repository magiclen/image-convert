use crate::{START_CALL_ONCE, ColorName, ImageResource, ImageConfig, compute_output_size_sharpen, magick_rust::{MagickWand, PixelWand, bindings}, starts_ends_with_caseless::EndsWithCaseless};

/// The output config of a PGM image.
pub struct PGMConfig {
    /// The width of the output image. `0` means the original width.
    pub width: u16,
    /// The height of the output image. `0` means the original height.
    pub height: u16,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only: bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
    /// The color is used for fill up the alpha background.
    pub background_color: Option<ColorName>,
}

impl PGMConfig {
    /// Create a `PGMConfig` instance with default values.
    /// ```rust,ignore
    /// PGMConfig {
    ///     width: 0u16,
    ///     height: 0u16,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    ///     background_color: None,
    /// }
    /// ```
    pub fn new() -> PGMConfig {
        PGMConfig {
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
            background_color: None,
        }
    }
}

impl ImageConfig for PGMConfig {
    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn get_sharpen(&self) -> f64 {
        self.sharpen
    }

    fn is_shrink_only(&self) -> bool {
        self.shrink_only
    }
}

/// Convert an image to a PGM image.
pub fn to_pgm(output: &mut ImageResource, input: &ImageResource, config: &PGMConfig) -> Result<(), &'static str> {
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

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);

    mw.profile_image("*", None)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("PGM")?;

    match output {
        ImageResource::Path(ref p) => {
            if !p.ends_with_caseless_ascii("pgm") {
                return Err("The file extension name is not pgm.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("PGM")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}