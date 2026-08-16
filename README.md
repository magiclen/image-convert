Image Convert
====================

[![CI](https://github.com/magiclen/image-convert/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/image-convert/actions/workflows/ci.yml)

This crate is a high level library using **MagickWand** (ImageMagick) for image identification, conversion, interlacing and high quality resizing.

## Examples

Identify an image.

```rust
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

```rust
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

An animated GIF, an animated WebP and a multi-page TIFF are read with all of their frames. `to_gif`, `to_webp` and `to_tiff` keep every frame, and each frame is cropped, resized and sharpened on its own. The other output formats store a single image, so they keep the first frame and drop the rest.

Two limitations are worth knowing:

* The GIF output is not layer-optimized, because **MagickWand** does not expose the layer optimization, so an animated GIF can come out considerably bigger than it went in.
* **ImageMagick** reads an animated PNG (APNG) through an external `ffmpeg` delegate only. Its built-in PNG decoder skips the animation and reads the first frame silently, so this crate treats an APNG as a still image. `ImageIdentify::has_unreadable_frames` reports when that happens.

## Orientation

Many cameras store a photo in the orientation of their sensor and put the real orientation into the metadata. `respect_orientation` rotates the image into that orientation and resets the metadata, so a viewer would not rotate it a second time. It defaults to `true`.

The orientation is also applied when `strip_metadata` is `true`, even if `respect_orientation` is `false`: the metadata is the only place where the orientation lives, so removing it without applying it first would leave the image lying on its side.

## Async

Every operation in this crate is CPU-bound. There is no I/O to wait for, so an `async fn` which does the work directly would hold an executor worker thread for the whole conversion and starve the other tasks. That is why the functions above are blocking.

To use this crate from async code, run it on a blocking thread pool. Enable the `tokio` feature to get the wrappers in the `asynchronous` module.

```toml
[dependencies]
image-convert = { version = "*", features = ["tokio"] }
```

```rust
use image_convert::{ImageResource, MagickError, PNGConfig, asynchronous::to_png};

async fn convert() -> Result<ImageResource, MagickError> {
    let input = ImageResource::from_path("tests/data/P1060382.JPG");
    let output = ImageResource::from_path("tests/data/P1060382_output.png");
    let mut config = PNGConfig::new();

    config.width = 1920;

    to_png(output, input, config).await
}
```

Without the `tokio` feature, wrapping the blocking functions in a thread of your own is straightforward. Note that `ImageResource` is `Send` but not `Sync`, because a `MagickWand` cannot be shared between threads, so it has to be moved into the closure instead of being borrowed.

```rust
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

## Crates.io

https://crates.io/crates/image-convert

## Documentation

https://docs.rs/image-convert

## License

[MIT](LICENSE)
