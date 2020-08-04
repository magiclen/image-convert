/*!
# Image Convert

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust,ignore
extern crate image_convert;

use image_convert::{ImageResource, InterlaceType, identify};

let input = ImageResource::from_path("tests/data/P1060382.JPG");

let mut output = None;

let id = identify(&mut output, &input).unwrap();

assert_eq!(4592, id.resolution.width);
assert_eq!(2584, id.resolution.height);
assert_eq!("JPEG", id.format);
assert_eq!(InterlaceType::NoInterlace, id.interlace);
```

Convert an image to a PNG image and also resize it.

```rust,ignore
extern crate image_convert;

use std::path::Path;

use image_convert::{ImageResource, PNGConfig, to_png};

let source_image_path = Path::new("tests/data/P1060382.JPG");

let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.png");

let mut config = PNGConfig::new();

config.width = 1920;

let input = ImageResource::from_path(source_image_path);

let mut output = ImageResource::from_path(target_image_path);

to_png(&mut output, &input, &config).unwrap();
```

Supported output formats are `BMP`, `JPG`, `PNG`, `GIF`, `WEBP`, `ICO`, `PGM` and `GrayRaw`.
*/

#![allow(clippy::enum_clike_unportable_variant)]

pub extern crate magick_rust;

#[macro_use]
extern crate enum_ordinalize;

extern crate ico;
extern crate once_cell;
extern crate regex;
extern crate str_utils;

#[cfg(feature = "none-background")]
macro_rules! set_none_background {
    ($mw:expr) => {{
        let mut pw = crate::magick_rust::PixelWand::new();
        pw.set_color("none")?;
        $mw.set_background_color(&pw)?;
    }};
}

#[cfg(not(feature = "none-background"))]
macro_rules! set_none_background {
    ($mw:expr) => {};
}

mod color_name;
mod crop;
mod format_bmp;
mod format_gif;
mod format_gray_raw;
mod format_ico;
mod format_jpeg;
mod format_pgm;
mod format_png;
mod format_tiff;
mod format_webp;
mod identify;
mod image_config;
mod image_resource;
mod interlace_type;

use std::cmp::Ordering;
use std::sync::Once;

pub use color_name::*;
pub use crop::*;
pub use format_bmp::*;
pub use format_gif::*;
pub use format_gray_raw::*;
pub use format_ico::*;
pub use format_jpeg::*;
pub use format_pgm::*;
pub use format_png::*;
pub use format_tiff::*;
pub use format_webp::*;
pub use identify::*;
pub(crate) use image_config::*;
pub use image_resource::*;
pub use interlace_type::InterlaceType;

use magick_rust::{magick_wand_genesis, MagickWand};

use once_cell::sync::Lazy;
use regex::Regex;

static START: Once = Once::new();

/// Call this function before using **MagickWand**.
pub const START_CALL_ONCE: fn() = || {
    START.call_once(|| {
        magick_wand_genesis();
    });
};

static RE_SVG: Lazy<Regex> = Lazy::new(|| Regex::new("(?i)(<svg[\\s\\S]*?>)").unwrap());
static RE_WIDTH: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i)([\\s\\S]*?[\\s]width[\\s]*=[\\s]*\"([\\s\\S]*?)\")").unwrap());
static RE_HEIGHT: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i)([\\s\\S]*?[\\s]height[\\s]*=[\\s]*\"([\\s\\S]*?)\")").unwrap());

pub fn fetch_magic_wand(
    input: &ImageResource,
    config: &impl ImageConfig,
) -> Result<(MagickWand, bool), &'static str> {
    START_CALL_ONCE();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image(p.as_str())?;

            if let Some(crop) = config.get_crop() {
                handle_crop(&mw, crop)?;
            }

            let format = mw.get_image_format()?;

            match format.as_str() {
                "SVG" | "MVG" => {
                    match compute_output_size_if_different(&mw, config) {
                        Some((new_width, new_height)) => {
                            let original_width = mw.get_image_width() as u16;

                            if new_width < original_width {
                                // TODO ImageMagick handles the smaller size of SVG poorly, so just do resize
                                Ok((mw, false))
                            } else {
                                use std::fs;

                                match fs::read_to_string(p) {
                                    Ok(svg) => {
                                        fetch_magic_wand_inner(mw, new_width, new_height, svg)
                                    }
                                    Err(_) => Ok((mw, false)),
                                }
                            }
                        }
                        None => Ok((mw, true)),
                    }
                }
                _ => Ok((mw, false)),
            }
        }
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image_blob(b)?;

            if let Some(crop) = config.get_crop() {
                handle_crop(&mw, crop)?;
            }

            let format = mw.get_image_format()?;

            match format.as_str() {
                "SVG" | "MVG" => {
                    match compute_output_size_if_different(&mw, config) {
                        Some((new_width, new_height)) => {
                            let original_width = mw.get_image_width() as u16;

                            if new_width < original_width {
                                // TODO ImageMagick handles the smaller size of SVG poorly, so just do resize
                                Ok((mw, false))
                            } else {
                                match String::from_utf8(b.to_vec()) {
                                    Ok(svg) => {
                                        fetch_magic_wand_inner(mw, new_width, new_height, svg)
                                    }
                                    Err(_) => Ok((mw, false)),
                                }
                            }
                        }
                        None => Ok((mw, true)),
                    }
                }
                _ => Ok((mw, false)),
            }
        }
        ImageResource::MagickWand(mw) => {
            let mw = mw.clone();

            if let Some(crop) = config.get_crop() {
                handle_crop(&mw, crop)?;
            }

            Ok((mw, false))
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn handle_crop(mw: &MagickWand, crop: Crop) -> Result<(), &'static str> {
    match crop {
        Crop::Center(w, h) => {
            let r = w / h;

            if r.is_nan() || r.is_infinite() || r == 0f64 {
                return Err("The ratio of CenterCrop is incorrect.");
            }

            let original_width = mw.get_image_width();
            let original_height = mw.get_image_height();

            let original_width_f64 = original_width as f64;
            let original_height_f64 = original_height as f64;

            let ratio = original_width_f64 / original_height_f64;

            let (new_width, new_height) = if r >= ratio {
                (original_width, (original_width_f64 / r).round() as usize)
            } else {
                ((original_height_f64 * r).round() as usize, original_height)
            };

            let x = (original_width - new_width) / 2;
            let y = (original_height - new_height) / 2;

            mw.crop_image(new_width, new_height, x as isize, y as isize)?;
        }
    }

    Ok(())
}

fn fetch_magic_wand_inner(
    mw: MagickWand,
    new_width: u16,
    new_height: u16,
    mut svg: String,
) -> Result<(MagickWand, bool), &'static str> {
    let result = match crate::RE_SVG.captures(&svg) {
        Some(captures) => {
            let target = captures.get(1).unwrap();

            let s = target.start() + 4;
            let mut e = target.end() - 1;

            let mut reload = false;

            let new_width = format!("{}px", new_width);
            let new_height = format!("{}px", new_height);

            let t = match crate::RE_WIDTH.captures(&svg[s..e]) {
                Some(captures) => {
                    let target = captures.get(2).unwrap();

                    let ts = target.start() + s;
                    let te = target.end() + s;

                    Some((ts, te))
                }
                None => None,
            };

            if let Some((ts, te)) = t {
                if svg[ts..te].ne(&new_width) {
                    svg.replace_range(ts..te, &new_width);

                    let tl = te - ts;
                    let l = new_height.len();

                    match l.cmp(&tl) {
                        Ordering::Greater => e += l - tl,
                        Ordering::Less => e -= tl - l,
                        Ordering::Equal => (),
                    }

                    reload = true;
                }
            }

            let t = match crate::RE_HEIGHT.captures(&svg[s..e]) {
                Some(captures) => {
                    let target = captures.get(2).unwrap();

                    let ts = target.start() + s;
                    let te = target.end() + s;

                    Some((ts, te))
                }
                None => None,
            };

            if let Some((ts, te)) = t {
                if svg[ts..te].ne(&new_height) {
                    svg.replace_range(ts..te, &new_height);

                    reload = true;
                }
            }

            if reload {
                let new_mw = MagickWand::new();

                set_none_background!(new_mw);

                match new_mw.read_image_blob(svg.into_bytes()) {
                    Ok(_) => (new_mw, true),
                    Err(_) => (mw, false),
                }
            } else {
                (mw, false)
            }
        }
        None => (mw, false),
    };

    Ok(result)
}
