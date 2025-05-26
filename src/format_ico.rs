use magick_rust::{FilterType, MagickError};
use str_utils::EndsWithIgnoreAsciiCase;

use crate::{compute_output_size_sharpen, fetch_magic_wand, Crop, ImageConfig, ImageResource};

#[derive(Debug)]
struct ICOConfigInner {
    strip_metadata:      bool,
    width:               u16,
    height:              u16,
    crop:                Option<Crop>,
    shrink_only:         bool,
    sharpen:             f64,
    respect_orientation: bool,
}

impl ICOConfigInner {
    pub fn from(config: &ICOConfig) -> Vec<ICOConfigInner> {
        let mut output = Vec::new();

        for (width, height) in config.size.iter().copied() {
            output.push(ICOConfigInner {
                strip_metadata: config.strip_metadata,
                width,
                height,
                crop: config.crop,
                shrink_only: false,
                sharpen: config.sharpen,
                respect_orientation: config.respect_orientation,
            });
        }

        output
    }
}

#[derive(Debug)]
/// The output config of an ICO image.
pub struct ICOConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The size of the output image, made up of a width and a height. `0` means the original width or the original height.
    pub size:                Vec<(u16, u16)>,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen:             f64,
    /// Apply orientation from image metadata if available.
    pub respect_orientation: bool,
}

impl ICOConfig {
    /// Create a `ICOConfig` instance with default values.
    /// ```rust,ignore
    /// ICOConfig {
    ///     strip_metadata: true,
    ///     size: Vec::with_capacity(1),
    ///     crop: None,
    ///     sharpen: -1f64,
    ///     respect_orientation: false,
    /// }
    /// ```
    #[inline]
    pub fn new() -> ICOConfig {
        ICOConfig {
            strip_metadata:      true,
            size:                Vec::with_capacity(1),
            crop:                None,
            sharpen:             -1f64,
            respect_orientation: false,
        }
    }
}

impl Default for ICOConfig {
    #[inline]
    fn default() -> Self {
        ICOConfig::new()
    }
}

impl ImageConfig for ICOConfigInner {
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

/// Convert an image to an ICO image.
pub fn to_ico(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &ICOConfig,
) -> Result<(), MagickError> {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    let ico_config_inner = ICOConfigInner::from(config);

    let mut config_iter = ico_config_inner.iter();

    let config = config_iter.next();

    if let Some(config) = config {
        let (mut mw, vector) = fetch_magic_wand(input, config)?;

        if vector {
            if config.strip_metadata {
                mw.strip_image()?;
            }

            mw.set_image_format("RGBA")?;
            mw.set_image_depth(8)?;

            {
                let temp = mw.write_image_blob("RGBA")?;

                let icon_image = ico::IconImage::from_rgba_data(
                    u32::from(config.width),
                    u32::from(config.height),
                    temp,
                );

                icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
            }

            for config in config_iter {
                let (mut mw, vector) = fetch_magic_wand(input, config)?;

                if !vector {
                    return Err("The input image may not be a correct vector.".into());
                }

                mw.strip_image()?;

                mw.set_image_format("RGBA")?;
                mw.set_image_depth(8)?;

                let temp = mw.write_image_blob("RGBA")?;

                let icon_image = ico::IconImage::from_rgba_data(
                    u32::from(config.width),
                    u32::from(config.height),
                    temp,
                );

                icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
            }
        } else {
            mw.strip_image()?;

            mw.set_image_format("RGBA")?;
            mw.set_image_depth(8)?;

            {
                let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

                mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;

                mw.sharpen_image(0f64, sharpen)?;

                let temp = mw.write_image_blob("RGBA")?;

                let icon_image =
                    ico::IconImage::from_rgba_data(u32::from(width), u32::from(height), temp);

                icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
            }

            for config in config_iter {
                let mw = mw.clone();

                let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

                mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;

                mw.sharpen_image(0f64, sharpen)?;

                let temp = mw.write_image_blob("RGBA")?;

                let icon_image =
                    ico::IconImage::from_rgba_data(u32::from(width), u32::from(height), temp);

                icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
            }
        }
    }

    match output {
        ImageResource::Path(p) => {
            if !p.ends_with_ignore_ascii_case_with_lowercase(".ico") {
                return Err("The file extension name is not ico.".into());
            }

            let file = match std::fs::File::create(p) {
                Ok(f) => f,
                Err(_) => return Err("Cannot create the icon file.".into()),
            };

            icon_dir.write(file).map_err(|_| "Cannot write the icon file.")?;
        },
        ImageResource::Data(b) => {
            icon_dir.write(b).map_err(|_| "Cannot convert to icon data.")?;
        },
        ImageResource::MagickWand(_) => {
            return Err("ICO cannot be output to a MagickWand instance.".into());
        },
    }

    Ok(())
}
