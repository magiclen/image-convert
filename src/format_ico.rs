use std::fs;

use magick_rust::{MagickError, MagickWand};

use crate::{
    Crop, ImageResource, compute_output_size,
    functions::{check_output, compute_crop_size, fetch_magic_wand_from_read, resize_and_sharpen},
    image_config::impl_image_config,
    read::read_image_wand,
    start_call_once,
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
    fn new(config: &ICOConfig, width: u32, height: u32) -> ICOConfigInner {
        ICOConfigInner {
            strip_metadata: config.strip_metadata,
            width,
            height,
            crop: config.crop,
            shrink_only: false,
            sharpen: config.sharpen,
            respect_orientation: config.respect_orientation,
        }
    }

    pub fn from(config: &ICOConfig) -> Vec<ICOConfigInner> {
        config
            .size
            .iter()
            .copied()
            .map(|(width, height)| Self::new(config, width, height))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
/// The output config of an ICO image.
pub struct ICOConfig {
    /// Remove the metadata stored in the input image.
    pub strip_metadata:      bool,
    /// The nonempty size limits of output images, made up of a width and a height; the aspect ratio is kept.
    /// `0` means no limit for that dimension, and `(0, 0)` keeps the original size.
    pub size:                Vec<(u32, u32)>,
    /// Crop the image.
    pub crop:                Option<Crop>,
    /// The sharpening strength; `0` disables sharpening, a negative value uses auto adjustment, and a positive value is used as given.
    /// Vector images rendered at the output size skip resizing and sharpening, even for positive values.
    /// If a vector image needs raster resizing, this setting applies as usual.
    pub sharpen:             f64,
    /// Apply orientation from image metadata if available. It is applied anyway when `strip_metadata` is `true`, because removing the metadata would otherwise throw the orientation away.
    pub respect_orientation: bool,
}

impl ICOConfig {
    /// Create an `ICOConfig` instance with default values.
    /// ```rust,ignore
    /// ICOConfig {
    ///     strip_metadata: true,
    ///     size: Vec::new(),
    ///     crop: None,
    ///     sharpen: -1f64,
    ///     respect_orientation: true,
    /// }
    /// ```
    #[inline]
    pub const fn new() -> ICOConfig {
        ICOConfig {
            strip_metadata:      true,
            size:                Vec::new(),
            crop:                None,
            sharpen:             -1f64,
            respect_orientation: true,
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
///
/// The output resource cannot be a `MagickWand` instance, because **MagickWand** does not encode the icon.
pub fn to_ico(
    output: &mut ImageResource,
    input: &ImageResource,
    config: &ICOConfig,
) -> Result<(), MagickError> {
    if let ImageResource::MagickWand(_) = output {
        return Err("ICO cannot be output to a MagickWand instance.".into());
    }

    check_output(output, &["ico"])?;

    let inner_configs = ICOConfigInner::from(config);

    let Some((last_config, rest_configs)) = inner_configs.split_last() else {
        return Err("The icon sizes cannot be empty.".into());
    };

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    start_call_once();
    let mw = read_image_wand(input, false, false)?;
    let mut first_index = 0;
    let mut source = None;

    if matches!(mw.get_image_format()?.as_str(), "SVG" | "MVG") {
        let (width, height) = match config.crop {
            Some(crop) => compute_crop_size(mw.get_image_width(), mw.get_image_height(), crop)?,
            None => (mw.get_image_width(), mw.get_image_height()),
        };

        // The largest size decides whether the input can be rendered as a vector image, no matter how the sizes are ordered.
        let mut largest_pixels = 0;
        for (index, config) in inner_configs.iter().enumerate() {
            let (width, height) = compute_output_size(
                false,
                width as u32,
                height as u32,
                config.width,
                config.height,
            )
            .unwrap_or((width as u32, height as u32));
            let pixels = u64::from(width) * u64::from(height);
            if pixels > largest_pixels {
                first_index = index;
                largest_pixels = pixels;
            }
        }

        // Keep the original rendering, so the other sizes do not need to read the input again.
        source = Some(mw.clone());
    }

    let (mut mw, vector) =
        fetch_magic_wand_from_read(mw, input, &inner_configs[first_index], None)?;

    match source {
        Some(source) if vector => {
            // A larger size is rendered from the vector image again, and a smaller size is resized from the original rendering.
            let mut first = Some(mw);

            for (index, config) in inner_configs.iter().enumerate() {
                let rendered = if index == first_index { first.take() } else { None };
                let (mut mw, vector) = match rendered {
                    Some(mw) => (mw, true),
                    None => fetch_magic_wand_from_read(source.clone(), input, config, None)?,
                };

                if !vector {
                    resize_and_sharpen(&mut mw, config)?;
                }

                add_icon_entry(&mut icon_dir, &mut mw, config.strip_metadata)?;
            }
        },
        _ => {
            // every size is resized from the original image, otherwise the later ones would be resized from another size
            for config in rest_configs {
                let mut mw = mw.clone();

                resize_and_sharpen(&mut mw, config)?;

                add_icon_entry(&mut icon_dir, &mut mw, config.strip_metadata)?;
            }

            // the last size does not need a clone anymore
            resize_and_sharpen(&mut mw, last_config)?;

            add_icon_entry(&mut icon_dir, &mut mw, last_config.strip_metadata)?;
        },
    }

    // the icon is encoded into memory first, because `IconDir::write` makes many small writes
    let mut data = Vec::new();

    icon_dir
        .write(&mut data)
        .map_err(|error| MagickError(format!("Cannot convert to icon data: {error}")))?;

    match output {
        ImageResource::Path(p) => {
            fs::write(p.as_str(), data)
                .map_err(|error| MagickError(format!("Cannot write the icon file: {error}")))?;
        },
        ImageResource::Data(b) => {
            *b = data;
        },
        ImageResource::MagickWand(_) => unreachable!(),
    }

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

    // `write_image_blob` sets the format itself
    mw.set_image_depth(8)?;

    let width = mw.get_image_width() as u32;
    let height = mw.get_image_height() as u32;

    let data = mw.write_image_blob("RGBA")?;

    // `IconImage::from_rgba_data` panics if the size or the length of the data is unexpected
    if width == 0 || height == 0 || data.len() as u64 != u64::from(width) * u64::from(height) * 4 {
        return Err("The image cannot be converted into an icon image.".into());
    }

    let icon_image = ico::IconImage::from_rgba_data(width, height, data);

    // a 256-pixel icon image is usually compressed as PNG since Windows Vista, and the readers before it cannot use such a size anyway
    let entry = if width >= 256 || height >= 256 {
        ico::IconDirEntry::encode_as_png(&icon_image)
    } else {
        ico::IconDirEntry::encode_as_bmp(&icon_image)
    };

    icon_dir.add_entry(entry.map_err(|_| "Cannot encode the icon image.")?);

    Ok(())
}
