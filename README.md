Image Convert
====================

[![Build Status](https://travis-ci.org/magiclen/image-convert.svg?branch=master)](https://travis-ci.org/magiclen/image-convert)
[![Build status](https://ci.appveyor.com/api/projects/status/a4kyteq8o64lydq5/branch/master?svg=true)](https://ci.appveyor.com/project/magiclen/image-convert/branch/master)

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust
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

```rust
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

Supported output formats are `JPG`, `PNG`, `GIF`, `WEBP`, `ICO`, `PGM` and `GrayRaw`.

## Crates.io

https://crates.io/crates/image-convert

## Documentation

https://docs.rs/image-convert

## License

[MIT](LICENSE)