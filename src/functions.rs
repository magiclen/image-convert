use std::{cmp::Reverse, fs, ops::Range, path::Path, str};

use magick_rust::{AlphaChannelOption, MagickError, MagickWand, PixelWand};

use crate::{
    Color, Crop, ImageConfig, ImageResource, image_config::compute_output_size_if_different,
    start_call_once,
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

/// Read an image from the input resource, and apply the orientation and the crop settings of the config to it.
///
/// The returned boolean indicates whether the image has already been rendered in the output size. If it is `true`, resizing the image again is unnecessary.
pub fn fetch_magic_wand(
    input: &ImageResource,
    config: &impl ImageConfig,
) -> Result<(MagickWand, bool), MagickError> {
    start_call_once();

    let mw = match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image(p.as_str())?;

            mw
        },
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image_blob(b)?;

            mw
        },
        ImageResource::MagickWand(mw) => mw.clone(),
    };

    // a vector image has to be re-rendered before being cropped, otherwise the crop result would be thrown away
    let (mw, vector) = if config.crop().is_none() {
        fetch_vector_magic_wand(mw, input, config)?
    } else {
        (mw, false)
    };

    if config.respect_orientation() && !mw.auto_orient() {
        return Err("Cannot apply the orientation of the image.".into());
    }

    if let Some(crop) = config.crop() {
        handle_crop(&mw, crop)?;
    }

    Ok((mw, vector))
}

// Re-render a vector image in the output size by rewriting its width and height, which keeps the image sharp.
fn fetch_vector_magic_wand(
    mw: MagickWand,
    input: &ImageResource,
    config: &impl ImageConfig,
) -> Result<(MagickWand, bool), MagickError> {
    match mw.get_image_format()?.as_str() {
        "SVG" | "MVG" => (),
        _ => return Ok((mw, false)),
    }

    let Some((new_width, new_height)) = compute_output_size_if_different(&mw, config) else {
        return Ok((mw, true));
    };

    if new_width < mw.get_image_width() as u32 {
        // TODO ImageMagick handles the smaller size of SVG poorly, so just do resize
        return Ok((mw, false));
    }

    match input {
        ImageResource::Path(p) => match fs::read_to_string(p) {
            Ok(svg) => resize_svg(mw, svg.as_str(), new_width, new_height),
            Err(_) => Ok((mw, false)),
        },
        ImageResource::Data(b) => match str::from_utf8(b) {
            Ok(svg) => resize_svg(mw, svg, new_width, new_height),
            Err(_) => Ok((mw, false)),
        },
        ImageResource::MagickWand(_) => Ok((mw, false)),
    }
}

fn resize_svg(
    mw: MagickWand,
    svg: &str,
    new_width: u32,
    new_height: u32,
) -> Result<(MagickWand, bool), MagickError> {
    let Some(tag) = find_svg_tag(svg) else {
        return Ok((mw, false));
    };

    let new_width = format!("{new_width}px");
    let new_height = format!("{new_height}px");

    let mut replacements = Vec::with_capacity(2);

    for (name, value) in [("width", new_width), ("height", new_height)] {
        if let Some(range) = find_attribute_value(&svg[tag.clone()], name) {
            let range = (tag.start + range.start)..(tag.start + range.end);

            if svg[range.clone()] != value {
                replacements.push((range, value));
            }
        }
    }

    if replacements.is_empty() {
        return Ok((mw, false));
    }

    // replace from the tail so that the ranges in front of the replaced one stay valid
    replacements.sort_by_key(|(range, _)| Reverse(range.start));

    let mut svg = svg.to_string();

    for (range, value) in replacements {
        svg.replace_range(range, value.as_str());
    }

    let new_mw = MagickWand::new();

    set_none_background!(new_mw);

    match new_mw.read_image_blob(svg.into_bytes()) {
        Ok(_) => Ok((new_mw, true)),
        Err(_) => Ok((mw, false)),
    }
}

// Find the range of the attribute part of the `<svg ...>` start tag.
fn find_svg_tag(svg: &str) -> Option<Range<usize>> {
    let bytes = svg.as_bytes();

    let mut index = 0;

    while index + 4 <= bytes.len() {
        if bytes[index] == b'<' && bytes[index + 1..index + 4].eq_ignore_ascii_case(b"svg") {
            let attributes_start = index + 4;

            // the tag name has to be exactly `svg`
            if attributes_start == bytes.len()
                || bytes[attributes_start].is_ascii_whitespace()
                || matches!(bytes[attributes_start], b'>' | b'/')
            {
                return find_tag_end(bytes, attributes_start).map(|end| attributes_start..end);
            }
        }

        index += 1;
    }

    None
}

// Find the `>` which closes a start tag, ignoring the one inside a quoted value.
fn find_tag_end(bytes: &[u8], mut index: usize) -> Option<usize> {
    while index < bytes.len() {
        match bytes[index] {
            b'>' => return Some(index),
            quote @ (b'"' | b'\'') => {
                index += 1;

                while index < bytes.len() && bytes[index] != quote {
                    index += 1;
                }
            },
            _ => (),
        }

        index += 1;
    }

    None
}

// Find the range of the value of the attribute with the given name. The name is compared in a case-insensitive way.
fn find_attribute_value(attributes: &str, name: &str) -> Option<Range<usize>> {
    let bytes = attributes.as_bytes();

    let mut index = 0;

    loop {
        while index < bytes.len()
            && (bytes[index].is_ascii_whitespace() || matches!(bytes[index], b'/' | b'?'))
        {
            index += 1;
        }

        let name_start = index;

        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && !matches!(bytes[index], b'=' | b'/')
        {
            index += 1;
        }

        if index == name_start {
            // there is nothing which can be parsed as an attribute name
            return None;
        }

        let matched = attributes[name_start..index].eq_ignore_ascii_case(name);

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if index == bytes.len() || bytes[index] != b'=' {
            // an attribute without a value
            continue;
        }

        index += 1;

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }

        if index == bytes.len() {
            return None;
        }

        let value_start;

        if let quote @ (b'"' | b'\'') = bytes[index] {
            index += 1;
            value_start = index;

            while index < bytes.len() && bytes[index] != quote {
                index += 1;
            }

            if matched {
                return Some(value_start..index);
            }

            index += 1;
        } else {
            value_start = index;

            while index < bytes.len() && !bytes[index].is_ascii_whitespace() {
                index += 1;
            }

            if matched {
                return Some(value_start..index);
            }
        }
    }
}

fn handle_crop(mw: &MagickWand, crop: Crop) -> Result<(), MagickError> {
    match crop {
        Crop::Center(w, h) => {
            let r = w / h;

            if r.is_nan() || r.is_infinite() || r <= 0f64 {
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

// Fill up the alpha background of the image with the given color.
pub(crate) fn handle_background_color(
    mw: &mut MagickWand,
    color: &Color,
) -> Result<(), MagickError> {
    let mut pw = PixelWand::new();

    pw.set_color(color.to_magick_color().as_ref())?;

    mw.set_image_background_color(&pw)?;
    mw.set_image_alpha_channel(AlphaChannelOption::Remove)?;

    Ok(())
}

// Write the image out to the output resource. `extensions` are the file extension names allowed by the output format.
pub(crate) fn write_output(
    output: &mut ImageResource,
    mw: MagickWand,
    extensions: &[&str],
    format: &str,
) -> Result<(), MagickError> {
    match output {
        ImageResource::Path(p) => {
            if !has_extension(p.as_str(), extensions) {
                return Err(MagickError(format!(
                    "The file extension name is not {}.",
                    extensions.join(" or ")
                )));
            }

            mw.write_image(p.as_str())?;
        },
        ImageResource::Data(b) => {
            b.append(&mut mw.write_image_blob(format)?);
        },
        ImageResource::MagickWand(mw_2) => {
            *mw_2 = mw;
        },
    }

    Ok(())
}

pub(crate) fn has_extension(path: &str, extensions: &[&str]) -> bool {
    match Path::new(path).extension() {
        Some(extension) => extensions.iter().any(|e| extension.eq_ignore_ascii_case(e)),
        None => false,
    }
}
