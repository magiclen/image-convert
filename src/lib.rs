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

## Async

Every operation in this crate is CPU-bound. There is no I/O to wait for, so an `async fn` which does the work directly would hold an executor worker thread for the whole conversion and starve the other tasks. That is why the functions above are blocking.

To use this crate from async code, run it on a blocking thread pool. Enable the `tokio` feature to get the wrappers in the `asynchronous` module.

```toml
[dependencies]
image-convert = { version = "0.21", features = ["tokio"] }
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
*/

#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "tokio")]
pub mod asynchronous;

mod color;
mod color_name;
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
