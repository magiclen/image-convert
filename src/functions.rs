use std::cmp::Ordering;

use magick_rust::{MagickError, MagickWand, OrientationType, PixelWand};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    image_config::compute_output_size_if_different, Crop, ImageConfig, ImageResource,
    START_CALL_ONCE,
};

#[cfg(feature = "none-background")]
macro_rules! set_none_background {
    ($mw:expr) => {{
        let mut pw = magick_rust::PixelWand::new();
        pw.set_color("none")?;
        $mw.set_background_color(&pw)?;
    }};
}

#[cfg(not(feature = "none-background"))]
macro_rules! set_none_background {
    ($mw:expr) => {};
}

pub(crate) use set_none_background;

static RE_SVG: Lazy<Regex> = Lazy::new(|| Regex::new("(?i)(<svg[\\s\\S]*?>)").unwrap());
static RE_WIDTH: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i)([\\s\\S]*?[\\s]width[\\s]*=[\\s]*\"([\\s\\S]*?)\")").unwrap());
static RE_HEIGHT: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i)([\\s\\S]*?[\\s]height[\\s]*=[\\s]*\"([\\s\\S]*?)\")").unwrap());

fn handle_orientation(mw: &MagickWand) -> Result<(), MagickError> {
    let orientation = mw.get_image_orientation();

    match orientation {
        // No rotation (normal)
        OrientationType::Undefined | OrientationType::TopLeft => (),
        // Horizontal flip
        OrientationType::TopRight => {
            mw.flop_image()?;
        },
        // Rotate 180°
        OrientationType::BottomRight => {
            mw.rotate_image(&PixelWand::new(), 180.0)?;
        },
        // Vertical flip
        OrientationType::BottomLeft => {
            mw.flip_image()?;
        },
        // Rotate 90° CCW + Vertical flip
        OrientationType::LeftTop => {
            mw.rotate_image(&PixelWand::new(), 270.0)?;
            mw.flip_image()?;
        },
        // Rotate 90° CW
        OrientationType::RightTop => {
            mw.rotate_image(&PixelWand::new(), 90.0)?;
        },
        // Rotate 90° CW + Vertical flip
        OrientationType::RightBottom => {
            mw.rotate_image(&PixelWand::new(), 90.0)?;
            mw.flip_image()?;
        },
        // Rotate 90° CCW
        OrientationType::LeftBottom => {
            mw.rotate_image(&PixelWand::new(), 270.0)?;
        },
    }

    Ok(())
}

pub fn fetch_magic_wand(
    input: &ImageResource,
    config: &impl ImageConfig,
) -> Result<(MagickWand, bool), MagickError> {
    START_CALL_ONCE();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image(p.as_str())?;

            if config.respect_orientation() {
                handle_orientation(&mw)?;
            }

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
                                    },
                                    Err(_) => Ok((mw, false)),
                                }
                            }
                        },
                        None => Ok((mw, true)),
                    }
                },
                _ => Ok((mw, false)),
            }
        },
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
                                    },
                                    Err(_) => Ok((mw, false)),
                                }
                            }
                        },
                        None => Ok((mw, true)),
                    }
                },
                _ => Ok((mw, false)),
            }
        },
        ImageResource::MagickWand(mw) => {
            let mw = mw.clone();

            if let Some(crop) = config.get_crop() {
                handle_crop(&mw, crop)?;
            }

            Ok((mw, false))
        },
    }
}

fn handle_crop(mw: &MagickWand, crop: Crop) -> Result<(), MagickError> {
    match crop {
        Crop::Center(w, h) => {
            let r = w / h;

            if r.is_nan() || r.is_infinite() || r == 0f64 {
                return Err("The ratio of CenterCrop is incorrect.".into());
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
        },
    }

    Ok(())
}

fn fetch_magic_wand_inner(
    mw: MagickWand,
    new_width: u16,
    new_height: u16,
    mut svg: String,
) -> Result<(MagickWand, bool), MagickError> {
    let result = match RE_SVG.captures(&svg) {
        Some(captures) => {
            let target = captures.get(1).unwrap();

            let s = target.start() + 4;
            let mut e = target.end() - 1;

            let mut reload = false;

            let new_width = format!("{new_width}px");
            let new_height = format!("{new_height}px");

            let t = match RE_WIDTH.captures(&svg[s..e]) {
                Some(captures) => {
                    let target = captures.get(2).unwrap();

                    let ts = target.start() + s;
                    let te = target.end() + s;

                    Some((ts, te))
                },
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

            let t = match RE_HEIGHT.captures(&svg[s..e]) {
                Some(captures) => {
                    let target = captures.get(2).unwrap();

                    let ts = target.start() + s;
                    let te = target.end() + s;

                    Some((ts, te))
                },
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
        },
        None => (mw, false),
    };

    Ok(result)
}
