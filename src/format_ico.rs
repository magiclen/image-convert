use crate::{
    compute_output_size_sharpen, fetch_magic_wand, magick_rust::bindings,
    starts_ends_with_caseless::EndsWithCaseless, ImageConfig, ImageResource,
};

#[derive(Debug)]
struct ICOConfigInner {
    remain_profile: bool,
    width: u16,
    height: u16,
    shrink_only: bool,
    sharpen: f64,
}

impl ICOConfigInner {
    pub fn from(config: &ICOConfig) -> Vec<ICOConfigInner> {
        let mut output = Vec::new();

        for (width, height) in config.size.iter().copied() {
            output.push(ICOConfigInner {
                remain_profile: config.remain_profile,
                width,
                height,
                shrink_only: false,
                sharpen: config.sharpen,
            });
        }

        output
    }
}

/// The output config of an ICO image.
pub struct ICOConfig {
    /// Remain the profile stored in the input image.
    pub remain_profile: bool,
    /// The size of the output image, made up of a width and a height. `0` means the original width or the original height.
    pub size: Vec<(u16, u16)>,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
}

impl ICOConfig {
    /// Create a `ICOConfig` instance with default values.
    /// ```rust,ignore
    /// ICOConfig {
    ///     remain_profile: false,
    ///     size: Vec::with_capacity(1),
    ///     sharpen: -1f64,
    /// }
    /// ```
    #[inline]
    pub fn new() -> ICOConfig {
        ICOConfig {
            remain_profile: false,
            size: Vec::with_capacity(1),
            sharpen: -1f64,
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

/// Convert an image to an ICO image.
pub fn to_ico(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &ICOConfig,
) -> Result<(), &'static str> {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    let ico_config_inner = ICOConfigInner::from(config);

    let mut config_iter = ico_config_inner.iter();

    let config = config_iter.next();

    if let Some(config) = config {
        let (mut mw, vector) = fetch_magic_wand(input, config)?;

        if vector {
            if !config.remain_profile {
                mw.profile_image("*", None)?;
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

            while let Some(config) = config_iter.next() {
                let (mut mw, vector) = fetch_magic_wand(input, config)?;

                if !vector {
                    return Err("The input image may not be a correct vector.");
                }

                mw.profile_image("*", None)?;

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
            mw.profile_image("*", None)?;

            mw.set_image_format("RGBA")?;
            mw.set_image_depth(8)?;

            {
                let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

                mw.resize_image(
                    width as usize,
                    height as usize,
                    bindings::FilterType_LanczosFilter,
                );

                mw.sharpen_image(0f64, sharpen)?;

                let temp = mw.write_image_blob("RGBA")?;

                let icon_image =
                    ico::IconImage::from_rgba_data(u32::from(width), u32::from(height), temp);

                icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
            }

            for config in config_iter {
                let mw = mw.clone();

                let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

                mw.resize_image(
                    width as usize,
                    height as usize,
                    bindings::FilterType_LanczosFilter,
                );

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
            if !p.ends_with_caseless_ascii(".ico") {
                return Err("The file extension name is not ico.");
            }

            let file = match std::fs::File::create(&p) {
                Ok(f) => f,
                Err(_) => return Err("Cannot create the icon file."),
            };

            if icon_dir.write(file).is_err() {
                return Err("Cannot write the icon file.");
            }
        }
        ImageResource::Data(b) => {
            if icon_dir.write(b).is_err() {
                return Err("Cannot convert to icon data.");
            }
        }
        ImageResource::MagickWand(_) => {
            return Err("ICO cannot be output to a MagickWand instance.");
        }
    }

    Ok(())
}
