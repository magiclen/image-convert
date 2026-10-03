use std::{io::Cursor, path::Path};

use image_convert::{
    BMPConfig, Color, GIFConfig, GrayRawConfig, ICOConfig, ImageResource, InterlaceType, JPGConfig,
    PGMConfig, PNGConfig, TIFFConfig, WEBPConfig, identify_ping, identify_read, to_bmp, to_gif,
    to_gray_raw, to_ico, to_jpg, to_pgm, to_png, to_tiff, to_webp,
};

const INPUT_IMAGE_PATH: &str = r"tests/data/dropbox.svg";
const INPUT_RECT_IMAGE_PATH: &str = r"tests/data/rect.svg";
// a 200x100 SVG image which has a width and a height but no `viewBox`
const INPUT_NO_VIEW_BOX_IMAGE_PATH: &str = r"tests/data/no_view_box.svg";

#[test]
fn get_identify() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let id = identify_ping(&input).unwrap();

    assert_eq!(512, id.resolution.width);
    assert_eq!(512, id.resolution.height);
    assert!(id.format == "MVG" || id.format == "SVG");
    assert_eq!(InterlaceType::No, id.interlace);
    assert!(id.has_alpha_channel);
}

#[test]
fn to_bmp_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.bmp");

    let mut config = BMPConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;
    config.background_color = Some(Color::GREEN);

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_bmp(&mut output, &input, &config).unwrap();
}

#[test]
fn to_jpg_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.jpg");

    let mut config = JPGConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_jpg(&mut output, &input, &config).unwrap();
}

#[test]
fn to_png_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.png");

    let mut config = PNGConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_png(&mut output, &input, &config).unwrap();
}

#[test]
fn to_png_file2file_small() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "dropbox_small_output.png");

    let mut config = PNGConfig::new();

    config.width = 16;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_png(&mut output, &input, &config).unwrap();
}

#[test]
fn to_png_file2file_rect() {
    let source_image_path = Path::new(INPUT_RECT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "rect_output.png");

    let mut config = PNGConfig::new();

    config.width = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let id = identify_ping(&ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(1920, id.resolution.width);
    assert_eq!(960, id.resolution.height);
}

#[test]
fn to_png_file2file_no_view_box() {
    let source_image_path = Path::new(INPUT_NO_VIEW_BOX_IMAGE_PATH);

    let target_image_path =
        Path::join(source_image_path.parent().unwrap(), "no_view_box_output.png");

    let mut config = PNGConfig::new();

    config.width = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(&target_image_path);

    to_png(&mut output, &input, &config).unwrap();

    let mut mw = None;

    let id = identify_read(&mut mw, &ImageResource::from_path(target_image_path)).unwrap();

    assert_eq!(1920, id.resolution.width);
    assert_eq!(960, id.resolution.height);

    // the content has to be scaled with the image instead of staying in the top-left corner
    let pixel = mw.unwrap().export_image_pixels(1919, 959, 1, 1, "RGBA").unwrap();

    assert_eq!([52, 152, 219, 255], pixel.as_slice());
}

#[test]
fn to_gif_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.gif");

    let mut config = GIFConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_gif(&mut output, &input, &config).unwrap();
}

#[test]
fn to_tiff_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.tif");

    let mut config = TIFFConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_tiff(&mut output, &input, &config).unwrap();
}

#[test]
fn to_webp_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.webp");

    let mut config = WEBPConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_webp(&mut output, &input, &config).unwrap();
}

#[test]
fn to_ico_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.ico");

    let mut config = ICOConfig::new();

    config.size.push((256u32, 256u32));
    config.size.push((16u32, 16u32));
    config.size.push((128u32, 128u32));
    config.size.push((64u32, 64u32));
    config.size.push((32u32, 32u32));

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_ico(&mut output, &input, &config).unwrap();
}

#[test]
fn to_ico_file2file_rect() {
    let source_image_path = Path::new(INPUT_RECT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "rect_output.ico");

    let mut config = ICOConfig::new();

    config.size.push((256u32, 256u32));
    config.size.push((32u32, 32u32));

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_ico(&mut output, &input, &config).unwrap();
}

#[test]
fn to_ico_data2data_size_order() {
    let mut ascending_config = ICOConfig::new();

    ascending_config.size.push((32u32, 32u32));
    ascending_config.size.push((1024u32, 1024u32));

    let mut descending_config = ICOConfig::new();

    descending_config.size.push((1024u32, 1024u32));
    descending_config.size.push((32u32, 32u32));

    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut ascending_output = ImageResource::Data(Vec::new());
    let mut descending_output = ImageResource::Data(Vec::new());

    to_ico(&mut ascending_output, &input, &ascending_config).unwrap();
    to_ico(&mut descending_output, &input, &descending_config).unwrap();

    let ascending = ico::IconDir::read(Cursor::new(ascending_output.into_vec().unwrap())).unwrap();
    let descending =
        ico::IconDir::read(Cursor::new(descending_output.into_vec().unwrap())).unwrap();

    assert_eq!(2, ascending.entries().len());
    assert_eq!(32, ascending.entries()[0].width());
    assert_eq!(1024, ascending.entries()[1].width());

    // the largest size has to be rendered from the vector image, no matter how the sizes are ordered
    assert!(descending.entries()[0].data() == ascending.entries()[1].data());
}

#[test]
fn to_gray_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.raw");

    let mut config = GrayRawConfig::new();

    config.width = 1920;
    config.height = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_gray_raw(&mut output, &input, &config).unwrap();
}

#[test]
fn to_pgm_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "dropbox_output.pgm");

    let mut config = PGMConfig::new();

    config.width = 1920;
    config.height = 1920;
    config.shrink_only = false;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_pgm(&mut output, &input, &config).unwrap();
}
