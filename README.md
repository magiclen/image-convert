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

## Async

Every operation in this crate is CPU-bound. There is no I/O to wait for, so an `async fn` which does the work directly would hold an executor worker thread for the whole conversion and starve the other tasks. That is why the functions above are blocking.

To use this crate from async code, run it on a blocking thread pool. Enable the `tokio` feature to get the wrappers in the `asynchronous` module.

```toml
[dependencies]
image-convert = { version = "0.21", features = ["tokio"] }
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
