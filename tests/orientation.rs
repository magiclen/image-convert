use std::{fs::File, path::Path};

use image_convert::{
    ImageResource, JPGConfig, PNGConfig, identify_ping, identify_read,
    magick_rust::OrientationType, to_jpg, to_png,
};

// a 200x100 JPEG image whose EXIF orientation is `RightTop`, which means it should be rotated 90 degrees clockwise
const INPUT_IMAGE_PATH: &str = r"tests/data/orientation.jpg";

#[test]
fn to_png_file2file_respect_orientation() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "orientation_output.png");

    let mut config = PNGConfig::new();

    config.respect_orientation = true;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let id = identify_ping(&ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(200, id.resolution.height);
}

#[test]
fn to_png_data2data_respect_orientation() {
    let input = ImageResource::from_reader(File::open(INPUT_IMAGE_PATH).unwrap()).unwrap();

    let mut config = PNGConfig::new();

    config.respect_orientation = true;

    let mut output = ImageResource::with_capacity(1024);

    to_png(&mut output, &input, &config).unwrap();

    let id = identify_ping(&ImageResource::Data(output.into_vec().unwrap())).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(200, id.resolution.height);
}

#[test]
fn to_png_file2file_ignore_orientation() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "orientation_ignored_output.png");

    let mut config = PNGConfig::new();

    config.respect_orientation = false;
    // the orientation is applied anyway when the metadata is stripped, because it would be lost otherwise
    config.strip_metadata = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let id = identify_ping(&ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(200, id.resolution.width);
    assert_eq!(100, id.resolution.height);
}

#[test]
fn to_png_file2file_strip_metadata_applies_orientation() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "orientation_stripped_output.png");

    let mut config = PNGConfig::new();

    config.respect_orientation = false;
    config.strip_metadata = true;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let id = identify_ping(&ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(200, id.resolution.height);
}

#[test]
fn to_png_file2file_keep_metadata_reset_orientation() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "orientation_kept_output.png");

    let mut config = PNGConfig::new();

    config.respect_orientation = true;
    config.strip_metadata = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let mut mw = None;

    let id = identify_read(&mut mw, &ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(200, id.resolution.height);

    // the EXIF profile is kept, so its orientation has to be reset as well, otherwise a viewer would rotate the output image again
    assert_eq!(OrientationType::TopLeft, mw.unwrap().get_image_orientation());
}

#[test]
fn to_jpg_file2file_reset_orientation() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "orientation_output.jpg");

    let mut config = JPGConfig::new();

    config.respect_orientation = true;
    config.strip_metadata = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_jpg(&mut output, &input, &config).unwrap();

    let mut mw = None;

    identify_read(&mut mw, &ImageResource::from_path(target_image_path)).unwrap();

    // the orientation has to be reset, otherwise a viewer would rotate the output image again
    assert_eq!(OrientationType::TopLeft, mw.unwrap().get_image_orientation());
}
