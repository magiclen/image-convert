//! # Image Convert
//! A library for image conversion, especially used for web applications.

extern crate magick_rust;
extern crate ico;

use std::sync::{Once, ONCE_INIT};
use std::path::Path;
use std::cmp;

use magick_rust::{MagickWand, magick_wand_genesis, PixelWand, bindings};

static START: Once = ONCE_INIT;

trait ImageConfig {
    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
    fn get_sharpen(&self) -> f64;
    fn is_shrink_only(&self) -> bool;
}

pub enum ImageResource<'a> {
    Path(&'a str),
    Data(Vec<u8>),
}

// TODO -----jpg START-----

pub struct JPGConfig {
    pub width: u16,
    pub height: u16,
    pub shrink_only: bool,
    pub sharpen: f64,
    pub force_to_chroma_quartered: bool,
    pub quality: u8,
    pub background_color: String,
    pub ppi: f64,
}

impl JPGConfig {
    pub fn new() -> JPGConfig {
        JPGConfig {
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
            force_to_chroma_quartered: true,
            quality: 85u8,
            background_color: String::new(),
            ppi: 72f64,
        }
    }
}

impl ImageConfig for JPGConfig {
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


pub fn to_jpg(output: &mut ImageResource, input: &ImageResource, config: &JPGConfig) -> Result<(), &'static str> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut mw = MagickWand::new();

    match input {
        ImageResource::Path(p) => {
            mw.read_image(p)?;
        }
        ImageResource::Data(ref b) => {
            mw.read_image_blob(b)?;
        }
    }

    if !config.background_color.is_empty() {
        let mut pw = PixelWand::new();
        pw.set_color(&config.background_color)?;
        mw.set_image_background_color(&pw)?;
        mw.set_image_alpha_channel(bindings::AlphaChannelOption::RemoveAlphaChannel)?;
    }

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType::LanczosFilter);

    mw.profile_image("*", None)?;

    if config.force_to_chroma_quartered {
        mw.set_sampling_factors(&[2f64, 1f64, 1f64])?;
    }

    mw.set_image_compression_quality(config.quality as usize)?;

    mw.set_image_interlace_scheme(bindings::InterlaceType::LineInterlace)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("JPEG")?;

    if config.ppi >= 0f64 {
        mw.set_image_resolution(config.ppi, config.ppi)?;
        mw.set_image_units(bindings::ResolutionType::PixelsPerInchResolution)?;
    }

    match output {
        ImageResource::Path(p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("jpg") && !file_name_lower_case.ends_with("jpeg") {
                return Err("The file extension name is not jpg or jpeg.");
            }

            mw.write_image(p)?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("JPEG")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}

// TODO -----jpg END-----

// TODO -----png START-----

pub struct PNGConfig {
    pub width: u16,
    pub height: u16,
    pub shrink_only: bool,
    pub sharpen: f64,
    pub ppi: f64,
}

impl PNGConfig {
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


pub fn to_png(output: &mut ImageResource, input: &ImageResource, config: &PNGConfig) -> Result<(), &'static str> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut mw = MagickWand::new();

    match input {
        ImageResource::Path(p) => {
            mw.read_image(p)?;
        }
        ImageResource::Data(ref b) => {
            mw.read_image_blob(b)?;
        }
    }

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType::LanczosFilter);

    mw.profile_image("*", None)?;

    mw.set_image_compression_quality(100)?;

    mw.set_image_interlace_scheme(bindings::InterlaceType::LineInterlace)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("PNG")?;

    if config.ppi >= 0f64 {
        mw.set_image_resolution(config.ppi, config.ppi)?;
        mw.set_image_units(bindings::ResolutionType::PixelsPerInchResolution)?;
    }

    match output {
        ImageResource::Path(ref p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("png") {
                return Err("The file extension name is not png.");
            }

            mw.write_image(p)?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("PNG")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}

// TODO -----png END-----

// TODO -----gif START-----

pub struct GIFConfig {
    pub width: u16,
    pub height: u16,
    pub shrink_only: bool,
    pub sharpen: f64,
}

impl GIFConfig {
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


pub fn to_gif(output: &mut ImageResource, input: &ImageResource, config: &GIFConfig) -> Result<(), &'static str> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut mw = MagickWand::new();

    match input {
        ImageResource::Path(p) => {
            mw.read_image(p)?;
        }
        ImageResource::Data(ref b) => {
            mw.read_image_blob(b)?;
        }
    }

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType::LanczosFilter);

    mw.profile_image("*", None)?;

    mw.set_image_compression_quality(100)?;

    mw.set_image_interlace_scheme(bindings::InterlaceType::LineInterlace)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("GIF")?;

    match output {
        ImageResource::Path(ref p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("gif") {
                return Err("The file extension name is not gif.");
            }

            mw.write_image(p)?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("GIF")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}

// TODO -----gif END-----

// TODO -----webp START-----

pub struct WEBPConfig {
    pub width: u16,
    pub height: u16,
    pub shrink_only: bool,
    pub sharpen: f64,
    pub quality: u8,
}

impl WEBPConfig {
    pub fn new() -> WEBPConfig {
        WEBPConfig {
            width: 0u16,
            height: 0u16,
            shrink_only: true,
            sharpen: -1f64,
            quality: 85u8,
        }
    }
}

impl ImageConfig for WEBPConfig {
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


pub fn to_webp(output: &mut ImageResource, input: &ImageResource, config: &WEBPConfig) -> Result<(), &'static str> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut mw = MagickWand::new();

    match input {
        ImageResource::Path(p) => {
            mw.read_image(p)?;
        }
        ImageResource::Data(ref b) => {
            mw.read_image_blob(b)?;
        }
    }

    let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

    mw.resize_image(width as usize, height as usize, bindings::FilterType::LanczosFilter);

    mw.profile_image("*", None)?;

    mw.set_image_compression_quality(config.quality as usize)?;

    mw.set_image_interlace_scheme(bindings::InterlaceType::LineInterlace)?;

    mw.sharpen_image(0f64, sharpen)?;

    mw.set_image_format("WEBP")?;

    match output {
        ImageResource::Path(ref p) => {
            let path = Path::new(&p);
            let file_name_lower_case = path.file_name().unwrap().to_str().unwrap().to_lowercase();

            if !file_name_lower_case.ends_with("webp") {
                return Err("The file extension name is not webp.");
            }

            mw.write_image(p)?;
        }
        ImageResource::Data(ref mut b) => {
            let mut temp = mw.write_image_blob("WEBP")?;
            b.append(&mut temp);
        }
    }

    Ok(())
}

// TODO -----webp END-----

// TODO -----gif START-----

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

pub struct ICOConfig {
    pub size: Vec<(u16, u16)>,
    pub sharpen: f64,
}

impl ICOConfig {
    pub fn new() -> ICOConfig {
        ICOConfig {
            size: Vec::new(),
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

pub fn to_ico(output: &mut ImageResource, input: &ImageResource, config: &ICOConfig) -> Result<(), &'static str> {
    START.call_once(|| {
        magick_wand_genesis();
    });

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for ref config in ICOConfigInner::from(&config) {
        let mut mw = MagickWand::new();

        match input {
            ImageResource::Path(p) => {
                mw.read_image(p)?;
            }
            ImageResource::Data(ref b) => {
                mw.read_image_blob(b)?;
            }
        }

        let (width, height, sharpen) = compute_output_size_sharpen(&mw, config);

        mw.resize_image(width as usize, height as usize, bindings::FilterType::LanczosFilter);

        mw.profile_image("*", None)?;

        mw.sharpen_image(0f64, sharpen)?;

        mw.set_image_format("RGBA")?;

        let temp = mw.write_image_blob("RGBA")?;

        let icon_image = ico::IconImage::from_rgba_data(width as u32, height as u32, temp);

        icon_dir.add_entry(ico::IconDirEntry::encode(&icon_image).unwrap());
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

// TODO -----ico END-----

fn compute_output_size_sharpen(mw: &MagickWand, config: &ImageConfig) -> (u16, u16, f64) {
    let mut width = config.get_width();
    let mut height = config.get_height();
    let original_width = mw.get_image_width() as u16;
    let original_height = mw.get_image_height() as u16;
    let ratio = original_width as f64 / original_height as f64;

    if config.is_shrink_only() {
        if width == 0 || width > original_width {
            width = original_width
        }
        if height == 0 || height > original_height {
            height = original_height
        }
    } else {
        if width == 0 {
            width = original_width
        }
        if height == 0 {
            height = original_height
        }
    }

    let wr = original_width as f64 / width as f64;
    let hr = original_height as f64 / height as f64;

    if wr >= hr {
        height = (width as f64 / ratio) as u16;
    } else {
        width = (height as f64 * ratio) as u16;
    }

    let mut adjusted_sharpen = config.get_sharpen();

    if adjusted_sharpen < 0f64 {
        let origin_pixels = original_width as u32 * original_height as u32;
        let resize_pixels = width as u32 * height as u32;
        let resize_level = (resize_pixels as f64 / 5000000f64).sqrt();

        let m = cmp::max(origin_pixels, resize_pixels) as f64;
        let n = cmp::min(origin_pixels, resize_pixels) as f64;

        adjusted_sharpen = resize_level * ((m - n) / m);

        if adjusted_sharpen < 0.1f64 {
            adjusted_sharpen = 0.1f64;
        } else if adjusted_sharpen > 3f64 {
            adjusted_sharpen = 3f64;
        }
    }

    (width, height, adjusted_sharpen)
}

// TODO -----Test START-----

#[cfg(test)]
mod tests {
    use super::*;

    use std::env;
    use std::path::Path;

    #[test]
    fn to_jpg_file2file() {
        let cwd = env::current_dir().unwrap();

        let source_image_path = Path::join(&cwd, "tests/data/P1060382.JPG");

        let target_image_path = Path::join(&cwd, "tests/data/P1060382_output.jpg");

        let mut config = JPGConfig::new();

        config.width = 1920;

        let input = ImageResource::Path(source_image_path.to_str().unwrap());

        let mut output = ImageResource::Path(target_image_path.to_str().unwrap());

        to_jpg(&mut output, &input, &config).unwrap();
    }

    #[test]
    fn to_png_file2file() {
        let cwd = env::current_dir().unwrap();

        let source_image_path = Path::join(&cwd, "tests/data/P1060382.JPG");

        let target_image_path = Path::join(&cwd, "tests/data/P1060382_output.png");

        let mut config = PNGConfig::new();

        config.width = 1920;

        let input = ImageResource::Path(source_image_path.to_str().unwrap());

        let mut output = ImageResource::Path(target_image_path.to_str().unwrap());

        to_png(&mut output, &input, &config).unwrap();
    }

    #[test]
    fn to_gif_file2file() {
        let cwd = env::current_dir().unwrap();

        let source_image_path = Path::join(&cwd, "tests/data/P1060382.JPG");

        let target_image_path = Path::join(&cwd, "tests/data/P1060382_output.gif");

        let mut config = GIFConfig::new();

        config.width = 1920;

        let input = ImageResource::Path(source_image_path.to_str().unwrap());

        let mut output = ImageResource::Path(target_image_path.to_str().unwrap());

        to_gif(&mut output, &input, &config).unwrap();
    }

    #[test]
    fn to_webp_file2file() {
        let cwd = env::current_dir().unwrap();

        let source_image_path = Path::join(&cwd, "tests/data/P1060382.JPG");

        let target_image_path = Path::join(&cwd, "tests/data/P1060382_output.webp");

        let mut config = WEBPConfig::new();

        config.width = 1920;

        let input = ImageResource::Path(source_image_path.to_str().unwrap());

        let mut output = ImageResource::Path(target_image_path.to_str().unwrap());

        to_webp(&mut output, &input, &config).unwrap();
    }

    #[test]
    fn to_ico_file2file() {
        let cwd = env::current_dir().unwrap();

        let source_image_path = Path::join(&cwd, "tests/data/P1060382.JPG");

        let target_image_path = Path::join(&cwd, "tests/data/P1060382_output.ico");

        let mut config = ICOConfig::new();

        config.size.push((256u16, 256u16));
        config.size.push((16u16, 16u16));
        config.size.push((128u16, 128u16));
        config.size.push((64u16, 64u16));
        config.size.push((32u16, 32u16));

        let input = ImageResource::Path(source_image_path.to_str().unwrap());

        let mut output = ImageResource::Path(target_image_path.to_str().unwrap());

        to_ico(&mut output, &input, &config).unwrap();
    }
}

// TODO -----Test END-----