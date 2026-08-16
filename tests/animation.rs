use std::path::Path;

use image_convert::{
    GIFConfig, ICOConfig, ImageResource, JPGConfig, PNGConfig, TIFFConfig, WEBPConfig,
    identify_ping, identify_read, to_gif, to_ico, to_jpg, to_png, to_tiff, to_webp,
};

// a 100x60 animated GIF made up of 4 frames, of which the last 3 are only 39x41 patches of the canvas
const INPUT_IMAGE_PATH: &str = r"tests/data/animation.gif";

// a 100x60 animated PNG, whose animation ImageMagick reads through an external delegate only
const INPUT_APNG_IMAGE_PATH: &str = r"tests/data/animation.png";

// a TIFF document made up of a 200x100 page and a 400x300 page
const INPUT_MULTIPAGE_IMAGE_PATH: &str = r"tests/data/multipage.tif";

#[test]
fn get_identify() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let id = identify_ping(&input).unwrap();

    // the size has to be the one of the first frame, not the one of the last patch
    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!("GIF", id.format);
    assert_eq!(4, id.number_of_frames);
    assert!(!id.has_unreadable_frames);
}

#[test]
fn get_identify_apng() {
    let input = ImageResource::from_path(INPUT_APNG_IMAGE_PATH);

    let id = identify_ping(&input).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!("PNG", id.format);
    assert_eq!(1, id.number_of_frames);
    assert!(id.has_unreadable_frames);
}

#[test]
fn to_gif_file2file_keeps_frames() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "animation_output.gif");

    let mut config = GIFConfig::new();

    config.width = 50;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_gif(&mut output, &input, &config).unwrap();

    let mut mw = None;

    let id = identify_read(&mut mw, &ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(50, id.resolution.width);
    assert_eq!(30, id.resolution.height);
    assert_eq!(4, id.number_of_frames);

    // every frame has to be composited and resized, not only the first one
    let mw = mw.unwrap();
    let images = mw.images();

    for index in 0..images.count() {
        let frame = images.get(index).unwrap();

        assert_eq!(50, frame.get_image_width());
        assert_eq!(30, frame.get_image_height());
    }
}

#[test]
fn to_gif_data2data_keeps_frames() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    to_gif(&mut output, &input, &GIFConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!(4, id.number_of_frames);
}

#[test]
fn to_gif_data2data_keeps_the_layer_optimization() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    // the config asks for no editing at all, so the frames must not be composited onto the canvas
    to_gif(&mut output, &input, &GIFConfig::new()).unwrap();

    let mut mw = None;

    let id = identify_read(&mut mw, &output).unwrap();

    assert_eq!(4, id.number_of_frames);

    let mw = mw.unwrap();
    let images = mw.images();

    // the frames after the first one are still patches instead of full-size frames
    for index in 1..images.count() {
        let frame = images.get(index).unwrap();

        assert_eq!(39, frame.get_image_width());
        assert_eq!(41, frame.get_image_height());
    }
}

#[test]
fn to_webp_data2data_keeps_frames() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    to_webp(&mut output, &input, &WEBPConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!(4, id.number_of_frames);
}

#[test]
fn to_png_data2data_keeps_the_first_frame() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    to_png(&mut output, &input, &PNGConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    // PNG stores a single image, so the first frame is the one which is kept
    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!(1, id.number_of_frames);
}

#[test]
fn to_jpg_data2data_keeps_the_first_frame() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    to_jpg(&mut output, &input, &JPGConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    assert_eq!(100, id.resolution.width);
    assert_eq!(60, id.resolution.height);
    assert_eq!(1, id.number_of_frames);
}

#[test]
fn to_ico_data2data() {
    let mut config = ICOConfig::new();

    config.size.push((32u32, 32u32));

    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(Vec::new());

    to_ico(&mut output, &input, &config).unwrap();

    let icon_dir = ico::IconDir::read(std::io::Cursor::new(output.into_vec().unwrap())).unwrap();

    assert_eq!(1, icon_dir.entries().len());
    assert_eq!(32, icon_dir.entries()[0].width());
}

#[test]
fn to_tiff_file2file_keeps_pages() {
    let source_image_path = Path::new(INPUT_MULTIPAGE_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "multipage_output.tif");

    let mut config = TIFFConfig::new();

    config.width = 100;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_tiff(&mut output, &input, &config).unwrap();

    let mut mw = None;

    let id = identify_read(&mut mw, &ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(2, id.number_of_frames);

    let mw = mw.unwrap();
    let images = mw.images();

    // every page keeps its own aspect ratio instead of being squeezed onto the canvas of the first one
    assert_eq!(100, images.get(0).unwrap().get_image_width());
    assert_eq!(50, images.get(0).unwrap().get_image_height());
    assert_eq!(100, images.get(1).unwrap().get_image_width());
    assert_eq!(75, images.get(1).unwrap().get_image_height());
}
