/*!
# Asynchronous API

Operations in this crate include CPU-heavy image processing, blocking file I/O and external delegates, so running them directly on an async executor would block a worker thread and delay other tasks. The functions in this module move the work onto Tokio's blocking thread pool with `spawn_blocking` instead.

Because `spawn_blocking` requires `'static`, these functions take owned values instead of references, and the output resource is given back in the returned value.
*/

use magick_rust::{MagickError, MagickWand};
use tokio::task::{JoinError, spawn_blocking};

use crate::{
    BMPConfig, GIFConfig, GrayRawConfig, ICOConfig, ImageIdentify, ImageResource, JPGConfig,
    PGMConfig, PNGConfig, TIFFConfig, WEBPConfig,
};

// The blocking task fails only when it panics or is cancelled.
#[inline]
fn handle_join_error(error: JoinError) -> MagickError {
    MagickError(error.to_string())
}

macro_rules! impl_async_convert {
    ($($(#[$attribute:meta])* $name:ident($config:ty)),+ $(,)?) => {
        $(
            $(#[$attribute])*
            pub async fn $name(
                mut output: ImageResource,
                input: ImageResource,
                config: $config,
            ) -> Result<ImageResource, MagickError> {
                spawn_blocking(move || {
                    crate::$name(&mut output, &input, &config)?;

                    Ok(output)
                })
                .await
                .map_err(handle_join_error)?
            }
        )+
    };
}

impl_async_convert! {
    /// Convert an image to a BMP image.
    to_bmp(BMPConfig),
    /// Convert an image to a GIF image.
    to_gif(GIFConfig),
    /// Convert an image to a RAW image with gray colors.
    to_gray_raw(GrayRawConfig),
    /// Convert an image to an ICO image.
    to_ico(ICOConfig),
    /// Convert an image to a JPEG image.
    to_jpg(JPGConfig),
    /// Convert an image to a PGM image.
    to_pgm(PGMConfig),
    /// Convert an image to a PNG image.
    to_png(PNGConfig),
    /// Convert an image to a TIFF image.
    to_tiff(TIFFConfig),
    /// Convert an image to a WEBP image.
    to_webp(WEBPConfig),
}

/// Ping and identify an image. It does not decode the pixels, so it is faster than `identify_read`.
pub async fn identify_ping(input: ImageResource) -> Result<ImageIdentify, MagickError> {
    spawn_blocking(move || crate::identify_ping(&input)).await.map_err(handle_join_error)?
}

/// Read and identify an image. It also gives back the `MagickWand` instance which holds the image.
pub async fn identify_read(
    input: ImageResource,
) -> Result<(ImageIdentify, MagickWand), MagickError> {
    spawn_blocking(move || {
        let mut output = None;

        let identify = crate::identify_read(&mut output, &input)?;

        match output {
            Some(mw) => Ok((identify, mw)),
            None => Err("Cannot read the image.".into()),
        }
    })
    .await
    .map_err(handle_join_error)?
}
