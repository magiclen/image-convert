/*!
# Image Convert

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust,no_run
use image_convert::{ImageResource, InterlaceType, MagickError, identify_read};

fn main() -> Result<(), MagickError> {
    let input = ImageResource::from_path("tests/data/P1060382.JPG");
    let mut output = None;
    let id = identify_read(&mut output, &input)?;

    assert_eq!(4592, id.resolution.width);
    assert_eq!(2584, id.resolution.height);
    assert_eq!("JPEG", id.format);
    assert_eq!(InterlaceType::No, id.interlace);

    Ok(())
}
```

Convert an image to a PNG image and also resize it.

```rust,no_run
use image_convert::{ImageResource, MagickError, PNGConfig, to_png};

fn main() -> Result<(), MagickError> {
    let mut config = PNGConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path("tests/data/P1060382.JPG");
    let mut output = ImageResource::from_path("tests/data/P1060382_output.png");

    to_png(&mut output, &input, &config)
}
```

Supported output formats are `BMP`, `JPG`, `PNG`, `GIF`, `TIFF`, `WEBP`, `ICO`, `PGM` and `GrayRaw`.

## Multi-frame images

Animated GIF, WebP and PNG (APNG) images and multi-page TIFF documents can be read with all of their frames. `to_gif`, `to_webp` and `to_tiff` keep the animation or pages. Animation frames are composited onto their canvas before editing or converting to another format, so transparent overlays and disposal rules are applied before cropping, resizing or removing metadata. TIFF pages keep their own sizes. They are not put onto a shared canvas when converted to GIF or WebP, so pages larger than the first one may be clipped. Single-image output formats keep the first displayed frame; for an APNG input they keep its default PNG image without running the animation delegate, and for an ICO or CUR input, which holds one picture in several sizes, they keep the largest one.

An unedited GIF converted to GIF keeps its original frame patches, even when the requested size equals the canvas size. WebP encoding needs complete frames, so its input is always composited. After compositing, GIF frames are not optimized again: **MagickWand** provides `MagickOptimizeImageLayers`, but the current `magick_rust` wrapper does not expose it. An edited GIF can therefore be larger than the input.

APNG decoding requires the **ImageMagick** APNG coder and an external `ffmpeg` delegate. Both file paths and in-memory input data are supported. The delegate uses temporary files, including for in-memory input. Decoding uses PAM to keep pixel data and transparency, then restores frame delays and the play count from the original PNG control chunks. Positive APNG delays are rounded to hundredths of a second, with a minimum of one hundredth. Output formats and ImageMagick may further limit timing precision or merge identical frames.

`identify_ping` does not run the APNG delegate: it reports the default PNG image, one frame and `has_unreadable_frames = true`. `identify_read` and conversions that keep frames decode the APNG animation or return an error if the delegate fails or the frame count is wrong. A `MagickWand` holding only the default APNG image cannot recover the missing frames; provide the original path or bytes instead. Successfully decoded APNG frames keep the `PNG` format name. APNG output is not provided.

Writing animated WebP requires **ImageMagick** built with `webpmux`; otherwise the conversion returns an error instead of writing a still image.

Multi-frame output to `ImageResource::Data` is encoded in a temporary directory and then read into memory to avoid an unchecked pointer in `magick_rust` when blob encoding fails. This requires writable temporary storage and adds file I/O; the directory is cleaned up when the operation ends. If encoding or reading fails, the original output data is kept.

## Quality

`JPGConfig::quality` can be `None`, which keeps the quality of the input image instead of asking for one. **ImageMagick** estimates that quality from the quantization tables of an input JPEG image, so re-encoding a JPEG image does not compress it a second time at a lower quality. It falls back to the default of **ImageMagick** when the input image is not a JPEG image.

## Color profiles

`strip_metadata` defaults to `true`. Images with an embedded ICC profile, such as Display P3 or Adobe RGB photos, are converted to sRGB before the profile is removed, so viewers which assume sRGB still show their colors correctly. Each frame is converted before compositing, resizing or filling its transparent background, and colors outside the sRGB gamut are mapped by the ICC conversion of **ImageMagick**. A profile which already matches sRGB is not applied again, and images without an ICC profile are not converted.

With `strip_metadata = false`, output formats which support ICC profiles keep the original profile and pixels. Background colors follow the color rules of **ImageMagick**, where RGB values and named colors are sRGB, so they are converted to the image profile before compositing. ICO, PGM and GrayRaw outputs cannot keep a profile, so they always convert to sRGB first, and the gray outputs then use the usual grayscale conversion.

ICC conversion requires **ImageMagick** built with the `lcms` delegate. A required conversion returns an error if it cannot run, rather than removing the profile and leaving the colors unchanged. A broken profile, or one made for other color channels, is ignored like browsers do, so the pixels keep their values.

## Orientation

Many cameras store a photo in the orientation of their sensor and put the real orientation into the metadata. `respect_orientation` rotates the image into that orientation and resets the metadata, so a viewer would not rotate it a second time. It defaults to `true`.

The orientation is also applied when `strip_metadata` is `true`, even if `respect_orientation` is `false`: the metadata is the only place where the orientation lives, so removing it without applying it first would leave the image lying on its side.

## Async

Operations in this crate include CPU-heavy image processing, blocking file I/O and external delegates. Running them directly in an `async fn` would block an executor worker thread and delay other tasks. That is why the functions above are blocking.

To use this crate from async code, run it on a blocking thread pool. Enable the `tokio` feature to get the wrappers in the `asynchronous` module.

Once a blocking task starts, dropping its future or timing out does not stop it.
Image processing, delegates and file writes can continue after the caller stops waiting.

```toml
[dependencies]
image-convert = { version = "*", features = ["tokio"] }
```

```rust,no_run
# #[cfg(feature = "tokio")]
use image_convert::{ImageResource, MagickError, PNGConfig, asynchronous::to_png};

# #[cfg(feature = "tokio")]
async fn convert() -> Result<ImageResource, MagickError> {
    let input = ImageResource::from_path("tests/data/P1060382.JPG");
    let output = ImageResource::from_path("tests/data/P1060382_output.png");
    let mut config = PNGConfig::new();

    config.width = 1920;

    to_png(output, input, config).await
}
```

Without the `tokio` feature, wrapping the blocking functions in a thread of your own is straightforward. Note that `ImageResource` is `Send` but not `Sync`, because a `MagickWand` cannot be shared between threads, so it has to be moved into the closure instead of being borrowed.

```rust,no_run
use image_convert::{ImageResource, MagickError, PNGConfig, to_png};

fn convert() -> Result<ImageResource, MagickError> {
    let input = ImageResource::from_path("tests/data/P1060382.JPG");
    let config = PNGConfig::new();
    let conversion = std::thread::spawn(move || {
        let mut output = ImageResource::Data(Vec::new());

        to_png(&mut output, &input, &config)?;

        Ok::<ImageResource, MagickError>(output)
    });

    match conversion.join() {
        Ok(output) => output,
        Err(_) => Err(MagickError("The conversion thread panicked.".to_owned())),
    }
}
```

Running conversions in parallel is safe, but keep in mind that ImageMagick already parallelizes internally with OpenMP and uses every core by default. Running many conversions at the same time oversubscribes the CPU, so limit the concurrency yourself, and consider `MagickWand::set_resource_limit(ResourceType::Thread, 1)` on Linux and macOS.

## Features

The `none-background` feature is enabled by default. It reads images with a transparent background color, so vector images such as SVG images keep their transparent areas. Without it, **ImageMagick** fills those areas with its default background color, which is white.

The `tokio` feature enables the `asynchronous` module described above.
*/

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "tokio")]
pub mod asynchronous;

mod color;
mod color_name;
mod color_profile;
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
mod functions;
mod identify;
mod image_config;
mod image_resource;
mod interlace_type;
mod read;

use std::sync::Once;

pub use color::*;
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
pub use functions::*;
pub use identify::*;
pub use image_config::*;
pub use image_resource::*;
pub use interlace_type::InterlaceType;
use magick_rust::magick_wand_genesis;
pub use magick_rust::{self, MagickError};

static START: Once = Once::new();

/// Initialize **MagickWand**. Every function in this crate calls it before using **MagickWand**, so usually there is no need to call it by yourself.
#[inline]
pub fn start_call_once() {
    START.call_once(magick_wand_genesis);
}
