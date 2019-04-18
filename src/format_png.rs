use std::path::Path;

use crate::{START_CALL_ONCE, InterlaceType, ImageResource, ImageConfig, compute_output_size_sharpen, magick_rust::{MagickWand, bindings}};

/// The output config of a PNG image.
pub struct PNGConfig {
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
    pub fn new() -> PNGConfig {
        PNGConfig {
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
            ppi: 72f64,
        }
    }
}

impl ImageConfig for PNGConfig {
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

/// Convert an image to a PNG image.
pub fn to_png(output: &mut ImageResource, input: &ImageResource, config: &PNGConfig) -> Result<(), &'static str> {
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

    mw.set_image_format("PNG")?;

    if config.ppi >= 0f64 {
        mw.set_image_resolution(config.ppi, config.ppi)?;
        mw.set_image_units(bindings::ResolutionType_PixelsPerInchResolution)?;
    }

    match output {
        ImageResource::Path(p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("png") {
                return Err("The file extension name is not png.");
            }

            mw.write_image(p.as_str())?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("PNG")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}