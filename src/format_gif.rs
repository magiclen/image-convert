use magick_rust::{FilterType, MagickError};
use str_utils::EndsWithIgnoreAsciiCase;

use crate::{
    compute_output_size_sharpen, fetch_magic_wand, Crop, ImageConfig, ImageResource, InterlaceType,
};

#[derive(Debug)]
/// The output config of a GIF image.
pub struct GIFConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The width of the output image. `0` means the original width.
    pub width:               u16,
    /// The height of the output image. `0` means the original height.
    pub height:              u16,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// Only shrink the image, not to enlarge it.
    pub shrink_only:         bool,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen:             f64,
    /// Apply orientation from image metadata if available.
    pub respect_orientation: bool,
}

impl GIFConfig {
    /// Create a `GIFConfig` instance with default values.
    /// ```rust,ignore
    /// GIFConfig {
    ///     strip_metadata: true,
    ///     width: 0u16,
    ///     height: 0u16,
    ///     crop: None,
    ///     shrink_only: true,
    ///     respect_orientation: false,
    ///     sharpen: -1f64,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> GIFConfig {
        GIFConfig {
            strip_metadata:      true,
            width:               0u16,
            height:              0u16,
            crop:                None,
            shrink_only:         true,
            respect_orientation: false,
            sharpen:             -1f64,
        }
    }
}

impl Default for GIFConfig {
    #[inline]
    fn default() -> Self {
        GIFConfig::new()
    }
}

impl ImageConfig for GIFConfig {
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
        self.sharpen
    }

    #[inline]
    fn is_shrink_only(&self) -> bool {
        self.shrink_only
    }

    #[inline]
    fn respect_orientation(&self) -> bool {
        self.respect_orientation
    }
}

/// Convert an image to a GIF image.
pub fn to_gif(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &GIFConfig,
) -> Result<(), MagickError> {
    let (mut mw, vector) = fetch_magic_wand(input, config)?;

    if !vector {
        let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;

        mw.sharpen_image(0f64, sharpen)?;
    }

    if config.strip_metadata {
        mw.strip_image()?;
    }

    mw.set_image_compression_quality(100)?;

    mw.set_interlace_scheme(InterlaceType::Line)?;

    mw.set_image_format("GIF")?;

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_ignore_ascii_case_with_lowercase(".gif") {
                return Err("The file extension name is not gif.".into());
            }

            mw.write_image(p.as_str())?;
        },
        ImageResource::Data(b) => {
            let mut temp = mw.write_image_blob("GIF")?;
            b.append(&mut temp);
        },
        ImageResource::MagickWand(mw_2) => {
            *mw_2 = mw;
        },
    }

    Ok(())
}
