use std::path::Path;

use crate::{START_CALL_ONCE, ImageResource, ImageConfig, compute_output_size_sharpen, magick_rust::{MagickWand, bindings}};

struct ICOConfigInner {
    width: u16,
    height: u16,
    shrink_only: bool,
    sharpen: f64,
}

impl ICOConfigInner {
    pub fn from(config: &ICOConfig) -> Vec<ICOConfigInner> {
        let mut output = Vec::new();

        for &(width, height) in &config.size {
            output.push(ICOConfigInner {
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
    /// The size of the output image, made up of a width and a height. `0` means the original width or the original height.
    pub size: Vec<(u16, u16)>,
    /// The higher the sharper. A negative value means auto adjustment.
    pub sharpen: f64,
}

impl ICOConfig {
    /// Create a `ICOConfig` instance with default values.
    /// ```rust,ignore
    /// ICOConfig {
    ///     size: Vec::with_capacity(1),
    ///     sharpen: -1f64,
    /// }
    /// ```
    pub fn new() -> ICOConfig {
        ICOConfig {
            size: Vec::with_capacity(1),
            sharpen: -1f64,
        }
    }
}

impl ImageConfig for ICOConfigInner {
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

/// Convert an image to an ICO image.
pub fn to_ico(output: &mut ImageResource, input: &ImageResource, config: &ICOConfig) -> Result<(), &'static str> {
    START_CALL_ONCE();

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for ref config in ICOConfigInner::from(&config) {
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

        mw.sharpen_image(0f64, sharpen)?;

        mw.set_image_format("RGBA")?;

        let temp = mw.write_image_blob("RGBA")?;

        let icon_image = ico::IconImage::from_rgba_data(width as u32, height as u32, temp);

        icon_dir.add_entry(ico::IconDirEntry::encode_as_bmp(&icon_image).unwrap());
    }

    match output {
        ImageResource::Path(ref p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("ico") {
                return Err("The file extension name is not ico.");
            }

            let file = match std::fs::File::create(&path) {
                Ok(f) => f,
                Err(_) => return Err("Cannot create the icon file.")
            };

            if let Err(_) = icon_dir.write(file) {
                return Err("Cannot write the icon file.");
            }
        }
        ImageResource::Data(ref mut b) => {
            if let Err(_) = icon_dir.write(b) {
                return Err("Cannot convert to icon data.");
            }
        }
    }

    Ok(())
}