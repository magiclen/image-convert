use crate::{START_CALL_ONCE, InterlaceType, ImageResource, {magick_rust::MagickWand}};

/// The resolution of an image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

/// The identified data of an image.
#[derive(Debug, Clone)]
pub struct ImageIdentify {
    pub resolution: Resolution,
    pub format: String,
    pub interlace: InterlaceType,
}

/// Identify an image. It can also be used for read an image as `MagickWand` instances.
pub fn identify(output: &mut Option<Vec<MagickWand>>, input: &ImageResource) -> Result<ImageIdentify, &'static str> {
    START_CALL_ONCE();

    let mw = MagickWand::new();

    if let None = output {
        match input {
            ImageResource::Path(p) => {
                mw.ping_image(p.as_str())?;
            }
            ImageResource::Data(ref b) => {
                mw.ping_image_blob(b)?;
            }
        }
    } else {
        match input {
            ImageResource::Path(p) => {
                mw.read_image(p.as_str())?;
            }
            ImageResource::Data(ref b) => {
                mw.read_image_blob(b)?;
            }
        }
    }

    let width = mw.get_image_width() as u32;

    let height = mw.get_image_height() as u32;

    let interlace = mw.get_image_interlace_scheme();

    let format = mw.get_image_format()?;

    let resolution = Resolution {
        width,
        height,
    };

    if let Some(s) = output {
        s.push(mw);
    }

    Ok(ImageIdentify {
        resolution,
        format,
        interlace: InterlaceType::from_ordinal(interlace as isize).unwrap_or(InterlaceType::UndefinedInterlace),
    })
}