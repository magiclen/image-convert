#![cfg(feature = "tokio")]

use std::{fs::File, path::Path};

use image_convert::{
    ImageResource, InterlaceType, PNGConfig,
    asynchronous::{identify_ping, to_png},
};

const INPUT_IMAGE_PATH: &str = r"tests/data/P1060382.JPG";

#[tokio::test]
async fn get_identify() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let id = identify_ping(input).await.unwrap();

    assert_eq!(4592, id.resolution.width);
    assert_eq!(2584, id.resolution.height);
    assert_eq!("JPEG", id.format);
    assert_eq!((180.0f64, 180.0f64), id.ppi);
    assert_eq!(InterlaceType::No, id.interlace);
    assert!(!id.has_alpha_channel);
}

#[tokio::test]
async fn to_png_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "P1060382_async_output.png");

    let mut config = PNGConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let output = ImageResource::from_path(target_image_path);

    to_png(output, input, config).await.unwrap();
}

#[tokio::test]
async fn to_png_data2data() {
    let input = ImageResource::from_reader(File::open(INPUT_IMAGE_PATH).unwrap()).unwrap();

    let mut config = PNGConfig::new();

    config.width = 1920;

    let output = ImageResource::with_capacity(1 << 20);

    let output = to_png(output, input, config).await.unwrap();

    let id = identify_ping(ImageResource::Data(output.into_vec().unwrap())).await.unwrap();

    assert_eq!(1920, id.resolution.width);
    assert_eq!(1080, id.resolution.height);
}
