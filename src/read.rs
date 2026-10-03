use std::{
    fs::File,
    io::{self, BufReader, Cursor, Read, Seek, SeekFrom},
};

use magick_rust::{DisposeType, MagickError, MagickWand};

use crate::{
    ImageResource,
    functions::{for_each_frame, set_none_background},
};

pub(crate) fn read_image_wand(
    input: &ImageResource,
    ping: bool,
    keep_frames: bool,
) -> Result<MagickWand, MagickError> {
    let mw = MagickWand::new();
    set_none_background!(mw);

    let mw = match input {
        ImageResource::Path(p) => {
            let path = png_path(p);
            if ping {
                mw.ping_image(path)?;
            } else {
                mw.read_image(path)?;
            }
            mw
        },
        ImageResource::Data(b) => {
            if ping {
                mw.ping_image_blob(b)?;
            } else {
                mw.read_image_blob(b)?;
            }
            mw
        },
        ImageResource::MagickWand(mw) => mw.clone(),
    };

    mw.reset_iterator();

    if ping || !keep_frames || mw.get_image_property("png:acTL").is_err() {
        return Ok(mw);
    }

    let metadata = match input {
        ImageResource::Path(p) => {
            File::open(png_path(p)).and_then(|file| read_apng_metadata(BufReader::new(file)))
        },
        ImageResource::Data(b) => read_apng_metadata(Cursor::new(b)),
        ImageResource::MagickWand(_) => {
            return Err("The APNG frames are missing; provide the original path or data to \
                        decode the animation."
                .into());
        },
    }
    .map_err(|error| MagickError(format!("Cannot read APNG animation metadata: {error}")))?;

    let mut animation = MagickWand::new();
    set_none_background!(animation);
    animation.set_format("APNG")?;
    // PAM keeps the alpha channel and avoids a lossy WebP step in the delegate.
    animation.set_option("video:intermediate-format", "pam")?;
    animation.set_option("video:vsync", "passthrough")?;
    animation.set_option(
        "video:pixel-format",
        if mw.get_image_depth() > 8 { "rgba64be" } else { "rgba" },
    )?;

    let decoded = match input {
        ImageResource::Path(p) => animation.read_image(&format!("APNG:{}", png_path(p))),
        ImageResource::Data(b) => animation.read_image_blob(b),
        ImageResource::MagickWand(_) => unreachable!(),
    };
    decoded.map_err(|error| {
        MagickError(format!(
            "Cannot decode the APNG animation through the ffmpeg delegate: {error}"
        ))
    })?;

    // The forced input format would otherwise override later blob output formats.
    animation.set_format("")?;

    if animation.get_number_images() != metadata.delays.len() {
        return Err("The APNG delegate did not decode every animation frame.".into());
    }

    let resolution = mw.get_image_resolution()?;
    let units = mw.get_image_units();
    let orientation = mw.get_image_orientation();
    let mut delays = metadata.delays.into_iter();

    for_each_frame(&mut animation, |frame| {
        // PAM has no timing data and uses ImageMagick's default time base of 100 ticks per second.
        frame.set_image_delay(delays.next().ok_or("The APNG frame timing is missing.")?)?;
        frame.set_image_iterations(metadata.plays as usize)?;
        frame.set_image_dispose(DisposeType::Background)?;
        frame.set_image_format("PNG")?;
        frame.set_image_resolution(resolution.0, resolution.1)?;
        frame.set_image_units(units)?;
        frame.set_image_orientation(orientation)?;
        Ok(())
    })?;

    Ok(animation)
}

fn png_path(path: &str) -> &str {
    match path.get(..5) {
        Some(prefix) if prefix.eq_ignore_ascii_case("apng:") => &path[5..],
        _ => path,
    }
}

struct ApngMetadata {
    plays:  u32,
    delays: Vec<usize>,
}

// Only read animation control data; ImageMagick and its delegate decode the pixels.
fn read_apng_metadata(mut reader: impl Read + Seek) -> io::Result<ApngMetadata> {
    let invalid =
        || io::Error::new(io::ErrorKind::InvalidData, "Invalid APNG animation control data.");
    let mut header = [0; 8];
    reader.read_exact(&mut header)?;
    if header != *b"\x89PNG\r\n\x1a\n" {
        return Err(invalid());
    }

    let mut count = None;
    let mut metadata = ApngMetadata {
        plays: 0, delays: Vec::new()
    };

    loop {
        reader.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap());
        match &header[4..] {
            b"acTL" => {
                if length != 8 || count.is_some() {
                    return Err(invalid());
                }
                let mut data = [0; 8];
                reader.read_exact(&mut data)?;
                count = Some(u32::from_be_bytes(data[..4].try_into().unwrap()) as usize);
                metadata.plays = u32::from_be_bytes(data[4..].try_into().unwrap());
            },
            b"fcTL" => {
                if length != 26 || count.is_none_or(|count| metadata.delays.len() >= count) {
                    return Err(invalid());
                }
                let mut data = [0; 26];
                reader.read_exact(&mut data)?;
                let numerator = u16::from_be_bytes([data[20], data[21]]) as usize;
                let denominator = u16::from_be_bytes([data[22], data[23]]) as usize;
                let denominator = if denominator == 0 { 100 } else { denominator };
                let delay = (numerator * 100 + denominator / 2) / denominator;
                metadata.delays.push(if numerator == 0 { 0 } else { delay.max(1) });
            },
            b"IEND" => {
                if length != 0 || metadata.delays.is_empty() || count != Some(metadata.delays.len())
                {
                    return Err(invalid());
                }
                return Ok(metadata);
            },
            _ => {
                reader.seek(SeekFrom::Current(i64::from(length)))?;
            },
        }
        reader.seek(SeekFrom::Current(4))?;
    }
}
