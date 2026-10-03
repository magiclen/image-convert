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

Animated GIF, WebP and PNG (APNG) images and multi-page TIFF documents can be read with all of their frames. `to_gif`, `to_webp` and `to_tiff` keep the animation or pages. Animation frames are composited onto their canvas before editing or converting to another format, so transparent overlays and disposal rules are applied before cropping, resizing or removing metadata. TIFF pages keep their own sizes. They are not put onto a shared canvas when converted to GIF or WebP, so pages larger than the first one may be clipped. Single-image output formats keep the first displayed frame; for an APNG input they keep its default PNG image without running the animation delegate, and for an ICO or CUR input, which holds one picture in several sizes, they keep the largest one.

An unedited GIF converted to GIF keeps its original frame patches, even when the requested size equals the canvas size. WebP encoding needs complete frames, so its input is always composited. After compositing, GIF frames are not optimized again: **MagickWand** provides `MagickOptimizeImageLayers`, but the current `magick_rust` wrapper does not expose it. An edited GIF can therefore be larger than the input.

APNG decoding requires the **ImageMagick** APNG coder and an external `ffmpeg` delegate. Both file paths and in-memory input data are supported. The delegate uses temporary files, including for in-memory input. Decoding uses PAM to keep pixel data and transparency, then restores frame delays and the play count from the original PNG control chunks. Positive APNG delays are rounded to hundredths of a second, with a minimum of one hundredth. Output formats and ImageMagick may further limit timing precision or merge identical frames.

`identify_ping` does not run the APNG delegate: it reports the default PNG image, one frame and `has_unreadable_frames = true`. `identify_read` and conversions that keep frames decode the APNG animation or return an error if the delegate fails or the frame count is wrong. A `MagickWand` holding only the default APNG image cannot recover the missing frames; provide the original path or bytes instead. Successfully decoded APNG frames keep the `PNG` format name. APNG output is not provided.

Writing animated WebP requires **ImageMagick** built with `webpmux`; otherwise the conversion returns an error instead of writing a still image.

## Quality

`JPGConfig::quality` can be `None`, which keeps the quality of the input image instead of asking for one. **ImageMagick** estimates that quality from the quantization tables of an input JPEG image, so re-encoding a JPEG image does not compress it a second time at a lower quality. It falls back to the default of **ImageMagick** when the input image is not a JPEG image.

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

## Features

The `none-background` feature is enabled by default. It reads images with a transparent background color, so vector images such as SVG images keep their transparent areas. Without it, **ImageMagick** fills those areas with its default background color, which is white.

The `tokio` feature enables the `asynchronous` module described above.

## Crates.io

https://crates.io/crates/image-convert

## Documentation

https://docs.rs/image-convert

## License

[MIT](LICENSE)
