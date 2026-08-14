use std::fs::File;

use magick_rust::{FilterType, MagickError, MagickWand};

use crate::{
    Crop, ImageResource, compute_output_size_sharpen, fetch_magic_wand, functions::has_extension,
    image_config::impl_image_config,
};

#[derive(Debug)]
struct ICOConfigInner {
    strip_metadata:      bool,
    width:               u32,
    height:              u32,
    crop:                Option<Crop>,
    shrink_only:         bool,
    sharpen:             f64,
    respect_orientation: bool,
}

impl ICOConfigInner {
    pub fn from(config: &ICOConfig) -> Vec<ICOConfigInner> {
        let mut output = Vec::with_capacity(config.size.len());

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

#[derive(Debug, Clone, PartialEq)]
/// The output config of an ICO image.
pub struct ICOConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The size of the output image, made up of a width and a height. `0` means the original width or the original height.
    pub size:                Vec<(u32, u32)>,
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

impl_image_config!(ICOConfigInner);

/// Convert an image to an ICO image.
pub fn to_ico(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &ICOConfig,
) -> Result<(), MagickError> {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    let inner_configs = ICOConfigInner::from(config);

    if let Some((first_config, rest_configs)) = inner_configs.split_first() {
        let (mut mw, vector) = fetch_magic_wand(input, first_config)?;

        if vector {
            // the input is a vector image, so render it in every size instead of resizing it
            add_icon_entry(&mut icon_dir, &mut mw, first_config.strip_metadata)?;

            for config in rest_configs {
                let (mut mw, vector) = fetch_magic_wand(input, config)?;

                if !vector {
                    // this size is smaller than the original size of the vector image
                    resize_icon_image(&mw, config)?;
                }

                add_icon_entry(&mut icon_dir, &mut mw, config.strip_metadata)?;
            }
        } else {
            // every size is resized from the original image, otherwise the later ones would be resized from another size
            for config in &inner_configs {
                let mut mw = mw.clone();

                resize_icon_image(&mw, config)?;

                add_icon_entry(&mut icon_dir, &mut mw, config.strip_metadata)?;
            }
        }
    }

    match output {
        ImageResource::Path(p) => {
            if !has_extension(p.as_str(), &["ico"]) {
                return Err("The file extension name is not ico.".into());
            }

            let file = match File::create(p) {
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

// Resize the image to the size set in the config.
fn resize_icon_image(mw: &MagickWand, config: &ICOConfigInner) -> Result<(), MagickError> {
    let (width, height, sharpen) = compute_output_size_sharpen(mw, config);

    mw.resize_image(width as usize, height as usize, FilterType::Lanczos)?;

    mw.sharpen_image(0f64, sharpen)?;

    Ok(())
}

// Encode the current image as an entry of the icon.
fn add_icon_entry(
    icon_dir: &mut ico::IconDir,
    mw: &mut MagickWand,
    strip_metadata: bool,
) -> Result<(), MagickError> {
    if strip_metadata {
        mw.strip_image()?;
    }

    mw.set_image_format("RGBA")?;
    mw.set_image_depth(8)?;

    let width = mw.get_image_width() as u32;
    let height = mw.get_image_height() as u32;

    let data = mw.write_image_blob("RGBA")?;

    // `IconImage::from_rgba_data` panics if the size or the length of the data is unexpected
    if width == 0 || height == 0 || data.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err("The image cannot be converted into an icon image.".into());
    }

    let icon_image = ico::IconImage::from_rgba_data(width, height, data);

    icon_dir.add_entry(
        ico::IconDirEntry::encode_as_bmp(&icon_image)
            .map_err(|_| "Cannot encode the icon image.")?,
    );

    Ok(())
}
