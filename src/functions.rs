use std::{cmp::Reverse, fs, ops::Range, path::Path, str};

use magick_rust::{
    AlphaChannelOption, FilterType, MagickError, MagickWand, OrientationType, PixelWand,
};

use crate::{
    Color, Crop, ImageConfig, ImageResource, compute_output_size,
    image_config::{compute_output_size_if_different, compute_output_size_sharpen},
    read::read_image_wand,
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
    fetch_magic_wand_inner(input, config, None)
}

pub(crate) fn fetch_magic_wand_for_format(
    input: &ImageResource,
    config: &impl ImageConfig,
    format: &str,
) -> Result<(MagickWand, bool), MagickError> {
    fetch_magic_wand_inner(input, config, Some(format))
}

fn fetch_magic_wand_inner(
    input: &ImageResource,
    config: &impl ImageConfig,
    format: Option<&str>,
) -> Result<(MagickWand, bool), MagickError> {
    start_call_once();

    let mut mw = read_image_wand(input, false, config.keep_frames())?;

    let keep_patches = prepare_frames(&mut mw, config, format)?;

    // a vector image has to be re-rendered before being cropped, otherwise the crop result would be thrown away
    let (mut mw, vector) = if config.crop().is_none() {
        fetch_vector_magic_wand(mw, input, config)?
    } else {
        (mw, false)
    };

    // stripping the metadata throws the orientation away, so the image has to be rotated before that happens
    if config.respect_orientation() || config.strip_metadata() {
        handle_orientation(&mut mw)?;
    }

    if let Some(crop) = config.crop() {
        handle_crop(&mut mw, crop)?;
    }

    Ok((mw, vector || keep_patches))
}

/// Run `f` on every frame of the image. The iterator of the wand is left on the first frame.
///
/// Almost every operation of **MagickWand** works on the current frame only, so it has to be repeated for a multi-frame image.
pub(crate) fn for_each_frame(
    mw: &mut MagickWand,
    mut f: impl FnMut(&mut MagickWand) -> Result<(), MagickError>,
) -> Result<(), MagickError> {
    mw.reset_iterator();

    while mw.next_image() {
        if let Err(error) = f(mw) {
            mw.reset_iterator();
            return Err(error);
        }
    }

    mw.reset_iterator();

    Ok(())
}

// Make the frames of the image ready to be edited one by one.
fn prepare_frames(
    mw: &mut MagickWand,
    config: &impl ImageConfig,
    format: Option<&str>,
) -> Result<bool, MagickError> {
    // reading an image leaves the iterator on the last frame instead of the first one
    mw.reset_iterator();

    if !config.keep_frames() && mw.get_number_images() > 1 {
        // Clone only the first image instead of removing the other frames by index.
        *mw = MagickWand::new_from_image(&mw.get_image()?)?;
    }

    let input_format = mw.get_image_format()?;

    // Document pages have separate sizes and must not share a canvas.
    if !matches!(input_format.as_str(), "GIF" | "WEBP" | "PNG" | "APNG" | "MNG") {
        return Ok(false);
    }

    let (page_width, page_height, x, y) = mw.get_image_page();
    let width = mw.get_image_width();
    let height = mw.get_image_height();
    let canvas_width = page_width.max(width.saturating_add(x.max(0) as usize));
    let canvas_height = page_height.max(height.saturating_add(y.max(0) as usize));

    if mw.get_number_images() == 1
        && canvas_width == width
        && canvas_height == height
        && x == 0
        && y == 0
    {
        return Ok(false);
    }

    let edits = config.crop().is_some()
        || config.sharpen() > 0f64
        || compute_output_size(
            config.shrink_only(),
            canvas_width as u32,
            canvas_height as u32,
            config.width(),
            config.height(),
        )
        .is_some()
        || ((config.respect_orientation() || config.strip_metadata()) && requires_orientation(mw));

    // WebP's encoder expects complete frames, even when the input is already WebP.
    let coalesce = !config.keep_frames()
        || edits
        || format.is_some_and(|format| format != input_format || format == "WEBP");

    if coalesce {
        *mw = mw.coalesce()?;
        mw.reset_iterator();
    }

    // Skip per-frame resizing when the logical canvas already has the requested size.
    Ok(!coalesce)
}

fn requires_orientation(mw: &MagickWand) -> bool {
    mw.reset_iterator();

    let mut required = false;
    while mw.next_image() {
        if !matches!(
            mw.get_image_orientation(),
            OrientationType::Undefined | OrientationType::TopLeft
        ) {
            required = true;
            break;
        }
    }

    mw.reset_iterator();
    required
}

// Rotate the image into the orientation which its metadata asks for, and reset that orientation so that a viewer would not rotate it again.
fn handle_orientation(mw: &mut MagickWand) -> Result<(), MagickError> {
    for_each_frame(mw, |frame| {
        // `auto_orient` clones the whole image even when there is nothing to do, and ImageMagick treats an undefined orientation as a top-left one
        if matches!(
            frame.get_image_orientation(),
            OrientationType::Undefined | OrientationType::TopLeft
        ) {
            return Ok(());
        }

        if !frame.auto_orient() {
            return Err("Cannot apply the orientation of the image.".into());
        }

        Ok(())
    })
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

    let width_value = format!("{new_width}px");
    let height_value = format!("{new_height}px");

    let mut replacements = Vec::with_capacity(2);

    for (name, value) in [("width", width_value), ("height", height_value)] {
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
        Ok(_) => {
            // the replaced attributes are not always the ones which decide the rendered size, so the result has to be checked
            let rendered = new_mw.get_image_width() as u32 == new_width
                && new_mw.get_image_height() as u32 == new_height;

            Ok((new_mw, rendered))
        },
        Err(_) => Ok((mw, false)),
    }
}

// Find the range of the attribute part of the `<svg ...>` start tag.
fn find_svg_tag(svg: &str) -> Option<Range<usize>> {
    let bytes = svg.as_bytes();

    let mut index = 0;

    while index + 4 <= bytes.len() {
        if bytes[index] == b'<' {
            // a comment or a CDATA section may hold something which looks like a start tag
            if let Some(end) = skip_ignorable_section(bytes, index) {
                index = end;

                continue;
            }

            if bytes[index + 1..index + 4].eq_ignore_ascii_case(b"svg") {
                let attributes_start = index + 4;

                // the tag name has to be exactly `svg`
                if attributes_start == bytes.len()
                    || bytes[attributes_start].is_ascii_whitespace()
                    || matches!(bytes[attributes_start], b'>' | b'/')
                {
                    return find_tag_end(bytes, attributes_start).map(|end| attributes_start..end);
                }
            }
        }

        index += 1;
    }

    None
}

// If a comment or a CDATA section starts at `index`, return the index right after its end, or the length of the input if it is never closed.
fn skip_ignorable_section(bytes: &[u8], index: usize) -> Option<usize> {
    const SECTIONS: [(&[u8], &[u8]); 2] = [(b"<!--", b"-->"), (b"<![CDATA[", b"]]>")];

    let rest = &bytes[index..];

    for (opening, closing) in SECTIONS {
        if rest.starts_with(opening) {
            let content_start = index + opening.len();

            return match find_bytes(&bytes[content_start..], closing) {
                Some(offset) => Some(content_start + offset + closing.len()),
                None => Some(bytes.len()),
            };
        }
    }

    None
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
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

fn handle_crop(mw: &mut MagickWand, crop: Crop) -> Result<(), MagickError> {
    match crop {
        Crop::Center(w, h) => {
            let r = w / h;

            if r.is_nan() || r.is_infinite() || r <= 0f64 {
                return Err("The ratio of CenterCrop is incorrect.".into());
            }

            for_each_frame(mw, |frame| {
                let original_width = frame.get_image_width();
                let original_height = frame.get_image_height();

                let original_width_f64 = original_width as f64;
                let original_height_f64 = original_height as f64;

                let ratio = original_width_f64 / original_height_f64;

                let (new_width, new_height) = if r >= ratio {
                    (original_width, (original_width_f64 / r).round() as usize)
                } else {
                    ((original_height_f64 * r).round() as usize, original_height)
                };

                let new_width = new_width.max(1).min(original_width);
                let new_height = new_height.max(1).min(original_height);

                let x = (original_width - new_width) / 2;
                let y = (original_height - new_height) / 2;

                frame.crop_image(new_width, new_height, x as isize, y as isize)?;

                // cropping keeps the original canvas size and the crop offset in the page geometry, which formats like GIF would store
                frame.reset_image_page("")?;

                Ok(())
            })?;
        },
    }

    Ok(())
}

// Resize the image to the size computed from the config, and then sharpen it.
pub(crate) fn resize_and_sharpen(
    mw: &mut MagickWand,
    config: &impl ImageConfig,
) -> Result<(), MagickError> {
    // the size is computed per frame, so the pages of a multi-page document keep their own aspect ratios
    for_each_frame(mw, |frame| {
        let (width, height, sharpen) = compute_output_size_sharpen(frame, config);

        // ImageMagick skips a resize with the same size only when the filter is undefined, so it has to be skipped here
        if width as usize != frame.get_image_width() || height as usize != frame.get_image_height()
        {
            frame.resize_image(width as usize, height as usize, FilterType::Lanczos)?;
        }

        if sharpen > 0f64 {
            frame.sharpen_image(0f64, sharpen)?;
        }

        Ok(())
    })
}

// Fill up the alpha background of the image with the given color.
pub(crate) fn handle_background_color(
    mw: &mut MagickWand,
    color: &Color,
) -> Result<(), MagickError> {
    let mut pw = PixelWand::new();

    pw.set_color(color.to_magick_color().as_ref())?;

    for_each_frame(mw, |frame| {
        frame.set_image_background_color(&pw)?;
        frame.set_image_alpha_channel(AlphaChannelOption::Remove)?;

        Ok(())
    })
}

// Write the image out to the output resource. `extensions` are the file extension names allowed by the output format.
pub(crate) fn write_output(
    output: &mut ImageResource,
    mw: MagickWand,
    extensions: &[&str],
    format: &str,
) -> Result<(), MagickError> {
    // `write_image` and `write_image_blob` store the current frame only
    let multi_frame = mw.get_number_images() > 1;

    if multi_frame && format == "WEBP" && !matches!(output, ImageResource::MagickWand(_)) {
        require_webp_animation()?;
    }

    match output {
        ImageResource::Path(p) => {
            if !has_extension(p.as_str(), extensions) {
                return Err(MagickError(format!(
                    "The file extension name is not {}.",
                    extensions.join(" or ")
                )));
            }

            if multi_frame {
                mw.write_images(p.as_str(), true)?;
            } else {
                mw.write_image(p.as_str())?;
            }
        },
        ImageResource::Data(b) => {
            let data = if multi_frame {
                mw.write_images_blob(format)?
            } else {
                mw.write_image_blob(format)?
            };

            // `write_images_blob` reports a failure as an empty blob instead of an error
            if data.is_empty() {
                return Err(MagickError(format!("Cannot write the image as {format} data.")));
            }

            *b = data;
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

fn require_webp_animation() -> Result<(), MagickError> {
    use magick_rust::bindings::{
        AcquireExceptionInfo, DestroyExceptionInfo, GetMagickAdjoin, GetMagickInfo,
        MagickBooleanType,
    };

    // SAFETY: ImageMagick is initialized, the name is a C string, and the exception is released after the borrowed coder is checked.
    let supported = unsafe {
        let exception = AcquireExceptionInfo();
        if exception.is_null() {
            return Err("Cannot check the WebP animation encoder.".into());
        }
        let coder = GetMagickInfo(c"WEBP".as_ptr(), exception);
        let supported = !coder.is_null() && GetMagickAdjoin(coder) == MagickBooleanType::MagickTrue;
        DestroyExceptionInfo(exception);
        supported
    };

    if supported {
        Ok(())
    } else {
        Err("Animated WebP output requires ImageMagick with the webpmux delegate.".into())
    }
}
