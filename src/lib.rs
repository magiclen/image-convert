/*!
# Image Convert

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust,ignore
use image_convert::{ImageResource, InterlaceType, identify};

let input = ImageResource::from_path("tests/data/P1060382.JPG");

let mut output = None;

let id = identify(&mut output, &input).unwrap();

assert_eq!(4592, id.resolution.width);
assert_eq!(2584, id.resolution.height);
assert_eq!("JPEG", id.format);
assert_eq!(InterlaceType::NoInterlace, id.interlace);
```

Convert an image to a PNG image and also resize it.

```rust,ignore
use std::path::Path;

use image_convert::{ImageResource, PNGConfig, to_png};

let source_image_path = Path::new("tests/data/P1060382.JPG");

let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.png");

let mut config = PNGConfig::new();

config.width = 1920;

let input = ImageResource::from_path(source_image_path);

let mut output = ImageResource::from_path(target_image_path);

to_png(&mut output, &input, &config).unwrap();
```

Supported output formats are `BMP`, `JPG`, `PNG`, `GIF`, `WEBP`, `ICO`, `PGM` and `GrayRaw`.
*/

pub extern crate magick_rust;

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
pub use magick_rust::MagickError;

static START: Once = Once::new();

/// Call this function before using **MagickWand**.
pub static START_CALL_ONCE: fn() = || {
    START.call_once(|| {
        magick_wand_genesis();
    });
};
