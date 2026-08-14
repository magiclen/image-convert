use magick_rust::{MagickError, MagickWand};

use crate::{ImageResource, InterlaceType, functions::set_none_background, start_call_once};

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
    /// The size of the image.
    pub resolution:        Resolution,
    /// The format of the image, such as `JPEG` or `PNG`.
    pub format:            String,
    /// The interlace scheme of the image.
    pub interlace:         InterlaceType,
    /// The horizontal and the vertical resolution in pixels per inch.
    pub ppi:               (f64, f64),
    /// Whether the image has an alpha channel.
    pub has_alpha_channel: bool,
}

fn identify_inner(mw: &MagickWand) -> Result<ImageIdentify, MagickError> {
    let width = mw.get_image_width() as u32;

    let height = mw.get_image_height() as u32;

    let resolution = Resolution {
        width,
        height,
    };

    let format = mw.get_image_format()?;

    let interlace = mw.get_image_interlace_scheme();

    let ppi = mw.get_image_resolution()?;

    let has_alpha_channel = mw.get_image_alpha_channel();

    Ok(ImageIdentify {
        resolution,
        format,
        interlace,
        ppi,
        has_alpha_channel,
    })
}

/// Ping and identify an image. It does not decode the pixels, so it is faster than `identify_read`.
pub fn identify_ping(input: &ImageResource) -> Result<ImageIdentify, MagickError> {
    start_call_once();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.ping_image(p.as_str())?;

            let identify = identify_inner(&mw)?;

            Ok(identify)
        },
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.ping_image_blob(b)?;

            let identify = identify_inner(&mw)?;

            Ok(identify)
        },
        ImageResource::MagickWand(mw) => {
            let identify = identify_inner(mw)?;

            Ok(identify)
        },
    }
}

/// Read and identify an image. It can read an image as `MagickWand` instances.
pub fn identify_read(
    output: &mut Option<MagickWand>,
    input: &ImageResource,
) -> Result<ImageIdentify, MagickError> {
    start_call_once();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image(p.as_str())?;

            let identify = identify_inner(&mw)?;

            output.replace(mw);

            Ok(identify)
        },
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

            set_none_background!(mw);

            mw.read_image_blob(b)?;

            let identify = identify_inner(&mw)?;

            output.replace(mw);

            Ok(identify)
        },
        ImageResource::MagickWand(mw) => {
            let identify = identify_inner(mw)?;

            output.replace(mw.clone());

            Ok(identify)
        },
    }
}
