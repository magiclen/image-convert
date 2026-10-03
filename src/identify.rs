use magick_rust::{MagickError, MagickWand, ResolutionType};

use crate::{ImageResource, InterlaceType, read::read_image_wand, start_call_once};

/// The resolution of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// The width in pixels.
    pub width:  u32,
    /// The height in pixels.
    pub height: u32,
}

/// The identified data of an image.
#[derive(Debug, Clone)]
pub struct ImageIdentify {
    /// The size of the image. It is the size of the first frame if the image has more than one.
    pub resolution:            Resolution,
    /// The format of the image, such as `JPEG` or `PNG`.
    pub format:                String,
    /// The interlace scheme of the image.
    pub interlace:             InterlaceType,
    /// The horizontal and the vertical resolution in pixels per inch.
    pub ppi:                   (f64, f64),
    /// Whether the image has an alpha channel.
    pub has_alpha_channel:     bool,
    /// The number of frames of the image, such as the frames of an animated GIF or the pages of a TIFF document.
    pub number_of_frames:      usize,
    /// Whether the image holds animation frames which were not decoded, in which case only the default PNG image is available.
    ///
    /// `identify_ping` does not run the APNG delegate and sets this to `true` for an APNG. `identify_read` decodes the animation through `ffmpeg` or returns an error.
    pub has_unreadable_frames: bool,
}

fn identify_inner(mw: &MagickWand) -> Result<ImageIdentify, MagickError> {
    // reading an image leaves the iterator on the last frame, which of an optimized animation is only a small patch
    mw.reset_iterator();

    let width = mw.get_image_width() as u32;

    let height = mw.get_image_height() as u32;

    let resolution = Resolution {
        width,
        height,
    };

    let format = mw.get_image_format()?;

    let interlace = mw.get_image_interlace_scheme();

    let mut ppi = mw.get_image_resolution()?;
    if mw.get_image_units() == ResolutionType::PixelsPerCentimeter {
        ppi.0 *= 2.54;
        ppi.1 *= 2.54;
    }

    let has_alpha_channel = mw.get_image_alpha_channel();

    let number_of_frames = mw.get_number_images();

    // the PNG decoder reports the animation control chunk of an APNG as a property, even though it cannot decode the frames
    let has_unreadable_frames = mw.get_image_property("png:acTL").is_ok();

    Ok(ImageIdentify {
        resolution,
        format,
        interlace,
        ppi,
        has_alpha_channel,
        number_of_frames,
        has_unreadable_frames,
    })
}

/// Ping and identify an image. It does not decode the pixels, so it is faster than `identify_read`.
pub fn identify_ping(input: &ImageResource) -> Result<ImageIdentify, MagickError> {
    start_call_once();

    match input {
        // the input holds the image already, so there is no need to clone it
        ImageResource::MagickWand(mw) => identify_inner(mw),
        _ => identify_inner(&read_image_wand(input, true, false)?),
    }
}

/// Read and identify an image. It can read an image as `MagickWand` instances.
pub fn identify_read(
    output: &mut Option<MagickWand>,
    input: &ImageResource,
) -> Result<ImageIdentify, MagickError> {
    start_call_once();

    let mw = read_image_wand(input, false, true)?;

    let identify = identify_inner(&mw)?;

    output.replace(mw);

    Ok(identify)
}
