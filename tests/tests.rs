use std::{io::Cursor, path::Path};

use image_convert::{
    BMPConfig, Crop, GIFConfig, GrayRawConfig, ICOConfig, ImageResource, InterlaceType, JPGConfig,
    PGMConfig, PNGConfig, TIFFConfig, WEBPConfig, identify_ping, identify_read,
    magick_rust::{MagickWand, PixelWand, ResolutionType},
    start_call_once, to_bmp, to_gif, to_gray_raw, to_ico, to_jpg, to_pgm, to_png, to_tiff, to_webp,
};

const INPUT_IMAGE_PATH: &str = r"tests/data/P1060382.JPG";

#[test]
fn get_identify() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let id = identify_ping(&input).unwrap();

    assert_eq!(4592, id.resolution.width);
    assert_eq!(2584, id.resolution.height);
    assert_eq!("JPEG", id.format);
    assert_eq!((180.0f64, 180.0f64), id.ppi);
    assert_eq!(InterlaceType::No, id.interlace);
    assert!(!id.has_alpha_channel);
}

#[test]
fn to_bmp_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.bmp");

    let mut config = BMPConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_bmp(&mut output, &input, &config).unwrap();
}

#[test]
fn to_jpg_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.jpg");

    let mut config = JPGConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_jpg(&mut output, &input, &config).unwrap();
}

#[test]
fn to_jpg_data2data_keeps_the_quality() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut input_mw = None;

    identify_read(&mut input_mw, &input).unwrap();

    let input_quality = input_mw.unwrap().get_image_compression_quality();

    let mut config = JPGConfig::new();

    config.quality = None;

    let mut output = ImageResource::Data(Vec::new());

    to_jpg(&mut output, &input, &config).unwrap();

    let mut output_mw = None;

    identify_read(&mut output_mw, &output).unwrap();

    // the quality ImageMagick estimated from the input image is the one it encodes with
    assert_eq!(input_quality, output_mw.unwrap().get_image_compression_quality());
}

#[test]
fn to_png_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.png");

    let mut config = PNGConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_png(&mut output, &input, &config).unwrap();
}

#[test]
fn to_png_data2data() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(b"old output".to_vec());

    to_png(&mut output, &input, &PNGConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    assert_eq!(4592, id.resolution.width);
    assert_eq!(2584, id.resolution.height);
    assert_eq!("PNG", id.format);
}

#[test]
fn wand_input_keeps_the_requested_output_format() {
    start_call_once();
    let mut color = PixelWand::new();
    color.set_color("red").unwrap();
    let mut mw = MagickWand::new();
    mw.new_image(2, 2, &color).unwrap();
    mw.set_image_format("PPM").unwrap();
    mw.set_format("PPM").unwrap();
    let input = ImageResource::MagickWand(mw);
    let mut output = ImageResource::Data(Vec::new());

    to_png(&mut output, &input, &PNGConfig::new()).unwrap();
    assert_eq!("PNG", identify_ping(&output).unwrap().format);

    let mut config = ICOConfig::new();
    config.size.push((2, 2));
    to_ico(&mut output, &input, &config).unwrap();
    let icon = ico::IconDir::read(Cursor::new(output.into_vec().unwrap())).unwrap();
    let image = icon.entries()[0].decode().unwrap();
    assert_eq!([255, 0, 0, 255].repeat(4), image.rgba_data());
    assert_eq!("PPM", input.as_magick_wand().unwrap().get_format().unwrap());
}

#[test]
fn to_png_file2wand_crop() {
    start_call_once();

    let mut config = PNGConfig::new();

    config.crop = Some(Crop::Center(1f64, 1f64));

    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::MagickWand(MagickWand::new());

    to_png(&mut output, &input, &config).unwrap();

    let mw = output.into_magick_wand().unwrap();

    assert_eq!(2584, mw.get_image_width());
    assert_eq!(2584, mw.get_image_height());
    assert_eq!((0, 0, 0, 0), mw.get_image_page());

    config.crop = Some(Crop::Center(10_000f64, 1f64));
    let mut output = ImageResource::MagickWand(MagickWand::new());
    to_png(&mut output, &input, &config).unwrap();
    let mw = output.into_magick_wand().unwrap();
    assert_eq!(4592, mw.get_image_width());
    assert_eq!(1, mw.get_image_height());
}

#[test]
fn identify_ppi_from_centimeters() {
    start_call_once();
    let mut mw = MagickWand::new();
    mw.new_image(1, 1, &PixelWand::new()).unwrap();
    mw.set_image_resolution(100.0, 50.0).unwrap();
    mw.set_image_units(ResolutionType::PixelsPerCentimeter).unwrap();
    let input = ImageResource::Data(mw.write_image_blob("TIFF").unwrap());

    assert_eq!((254.0, 127.0), identify_ping(&input).unwrap().ppi);
    assert_eq!((254.0, 127.0), identify_read(&mut None, &input).unwrap().ppi);
}

#[test]
fn to_gray_raw_converts_colors() {
    start_call_once();
    let mut mw = MagickWand::new();
    mw.new_image(3, 1, &PixelWand::new()).unwrap();
    mw.import_image_pixels(0, 0, 3, 1, &[255, 0, 0, 0, 255, 0, 0, 0, 255], "RGB").unwrap();
    let input = ImageResource::MagickWand(mw);
    let mut output = ImageResource::Data(Vec::new());

    to_gray_raw(&mut output, &input, &GrayRawConfig::new()).unwrap();

    assert_eq!([54, 182, 18], output.into_vec().unwrap().as_slice());
}

#[test]
fn to_gif_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.gif");

    let mut config = GIFConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_gif(&mut output, &input, &config).unwrap();
}

#[test]
fn to_tiff_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.tif");

    let mut config = TIFFConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_tiff(&mut output, &input, &config).unwrap();
}

#[test]
fn to_webp_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.webp");

    let mut config = WEBPConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_webp(&mut output, &input, &config).unwrap();
}

#[test]
fn to_ico_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.ico");

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
fn to_ico_data2data() {
    let mut config = ICOConfig::new();

    config.size.push((32u32, 32u32));

    let input = ImageResource::from_path(INPUT_IMAGE_PATH);

    let mut output = ImageResource::Data(b"old output".to_vec());

    to_ico(&mut output, &input, &config).unwrap();

    let icon_dir = ico::IconDir::read(Cursor::new(output.into_vec().unwrap())).unwrap();

    assert_eq!(1, icon_dir.entries().len());
}

#[test]
fn to_png_file2data_keeps_the_largest_icon_image() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let icon_image_path =
        Path::join(source_image_path.parent().unwrap(), "P1060382_sizes_output.ico");

    let mut config = ICOConfig::new();

    config.size.push((16u32, 16u32));
    config.size.push((64u32, 64u32));

    let mut icon = ImageResource::from_path(&icon_image_path);

    to_ico(&mut icon, &ImageResource::from_path(source_image_path), &config).unwrap();

    // ImageMagick cannot detect an icon from its data, so the icon is read from its path
    let input = ImageResource::from_path(icon_image_path);

    let mut output = ImageResource::Data(Vec::new());

    to_png(&mut output, &input, &PNGConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();

    // the icon holds the same picture in several sizes, so the largest one is kept instead of the first one
    assert_eq!(64, id.resolution.width);
    assert_eq!(36, id.resolution.height);
}

#[test]
fn to_gray_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.raw");

    let mut config = GrayRawConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_gray_raw(&mut output, &input, &config).unwrap();
}

#[test]
fn to_pgm_file2file() {
    let source_image_path = Path::new(INPUT_IMAGE_PATH);

    let target_image_path = Path::join(source_image_path.parent().unwrap(), "P1060382_output.pgm");

    let mut config = PGMConfig::new();

    config.width = 1920;

    let input = ImageResource::from_path(source_image_path);

    let mut output = ImageResource::from_path(target_image_path);

    to_pgm(&mut output, &input, &config).unwrap();
}
