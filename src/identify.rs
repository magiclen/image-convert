use crate::{magick_rust::MagickWand, ImageResource, InterlaceType, MagickError, START_CALL_ONCE};

/// The resolution of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub width:  u32,
    pub height: u32,
}

/// The identified data of an image.
#[derive(Debug, Clone)]
pub struct ImageIdentify {
    pub resolution: Resolution,
    pub format:     String,
    pub interlace:  InterlaceType,
    pub ppi:        (f64, f64),
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

    Ok(ImageIdentify {
        resolution,
        format,
        interlace: InterlaceType::from_ordinal(interlace as isize)
            .unwrap_or(InterlaceType::UndefinedInterlace),
        ppi,
    })
}

#[deprecated(since = "0.11.3", note = "please use `identify_ping` or `identify_read` instead")]
/// Identify an image. It can also be used for read an image as `MagickWand` instances.
#[allow(clippy::option_option)]
pub fn identify(
    output: &mut Option<Option<MagickWand>>,
    input: &ImageResource,
) -> Result<ImageIdentify, MagickError> {
    START_CALL_ONCE();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            if output.is_some() {
                set_none_background!(mw);

                mw.read_image(p.as_str())?;
            } else {
                mw.ping_image(p.as_str())?;
            }

            let identify = identify_inner(&mw)?;

            if let Some(s) = output {
                s.replace(mw);
            }

            Ok(identify)
        },
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

            if output.is_some() {
                set_none_background!(mw);

                mw.read_image_blob(b)?;
            } else {
                mw.ping_image_blob(b)?;
            }

            let identify = identify_inner(&mw)?;

            if let Some(s) = output {
                s.replace(mw);
            }

            Ok(identify)
        },
        ImageResource::MagickWand(mw) => {
            let identify = identify_inner(mw)?;

            if let Some(s) = output {
                s.replace(mw.clone());
            }

            Ok(identify)
        },
    }
}

/// Ping and identify an image.
pub fn identify_ping(input: &ImageResource) -> Result<ImageIdentify, MagickError> {
    START_CALL_ONCE();

    match input {
        ImageResource::Path(p) => {
            let mw = MagickWand::new();

            mw.ping_image(p.as_str())?;

            let identify = identify_inner(&mw)?;

            Ok(identify)
        },
        ImageResource::Data(b) => {
            let mw = MagickWand::new();

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
    START_CALL_ONCE();

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
