use crate::{START_CALL_ONCE, InterlaceType, ImageResource, ImageConfig, compute_output_size_sharpen, magick_rust::{MagickWand, bindings}, starts_ends_with_caseless::EndsWithCaseless};

/// The output config of a GIF image.
pub struct GIFConfig {
    /// The width of the output image. `0` means the original width.
    pub width: u16,
    /// The height of the output image. `0` means the original height.
    pub height: u16,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only: bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
}

impl GIFConfig {
    /// Create a `GIFConfig` instance with default values.
    /// ```rust,ignore
    /// GIFConfig {
    ///     width: 0u16,
    ///     height: 0u16,
    ///     shrink_only: true,
    ///     sharpen: -1f64,
    /// }
    /// ```
    pub fn new() -> GIFConfig {
        GIFConfig {
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
        }
    }
}

impl ImageConfig for GIFConfig {
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

/// Convert an image to a GIF image.
pub fn to_gif(output: &mut ImageResource, input: &ImageResource, config: &GIFConfig) -> Result<(), &'static str> {
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

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType_LanczosFilter);

    mw.profile_image("*", None)?;

    mw.set_image_compression_quality(100)?;

    mw.set_interlace_scheme(InterlaceType::LineInterlace.ordinal() as bindings::InterlaceType)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("GIF")?;

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_caseless_ascii(".gif") {
                return Err("The file extension name is not gif.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("GIF")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}