/*!
# Image Convert

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust,ignore
extern crate image_convert;

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
extern crate image_convert;

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

Supported output formats are `JPG`, `PNG`, `GIF`, `WEBP`, `ICO` and `GrayRaw`.
*/

pub extern crate magick_rust;
#[macro_use]
extern crate enum_ordinalize;
extern crate ico;
extern crate starts_ends_with_caseless;

mod color_name;
mod interlace_type;
mod image_resource;
mod image_config;
mod identify;
mod format_jpeg;
mod format_png;
mod format_gif;
mod format_webp;
mod format_ico;
mod format_gray_raw;
mod format_pgm;

use std::sync::{Once, ONCE_INIT};

pub use color_name::*;
pub use interlace_type::InterlaceType;
pub use self::image_resource::*;
pub(crate) use self::image_config::*;
pub use self::identify::*;
pub use self::format_jpeg::*;
pub use self::format_png::*;
pub use self::format_gif::*;
pub use self::format_webp::*;
pub use self::format_ico::*;
pub use self::format_gray_raw::*;
pub use self::format_pgm::*;

use magick_rust::magick_wand_genesis;

static START: Once = ONCE_INIT;

/// Call this function before using **MagickWand**.
pub const START_CALL_ONCE: fn() = || {
    START.call_once(|| {
        magick_wand_genesis();
    });
};