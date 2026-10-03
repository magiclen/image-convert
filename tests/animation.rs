use std::{fs, path::Path};

use image_convert::{
    Crop, GIFConfig, ICOConfig, ImageResource, JPGConfig, PNGConfig, TIFFConfig, WEBPConfig,
    identify_ping, identify_read,
    magick_rust::{DisposeType, MagickWand, PixelWand},
    start_call_once, to_gif, to_ico, to_jpg, to_png, to_tiff, to_webp,
};

// a 100x60 animated GIF made up of 4 frames, of which the last 3 are only 39x41 patches of the canvas
const INPUT_IMAGE_PATH: &str = r"tests/data/animation.gif";

// a 100x60 animated PNG, whose animation ImageMagick reads through an external delegate only
const INPUT_APNG_IMAGE_PATH: &str = r"tests/data/animation.png";

// The same APNG with delays of 100, 200, 300 and 400 ms and three plays.
const INPUT_TIMED_APNG_IMAGE_PATH: &str = r"tests/data/animation_timing.png";

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
fn identify_ping_keeps_wand_iterator() {
    let mut mw = None;
    identify_read(&mut mw, &ImageResource::from_path(INPUT_IMAGE_PATH)).unwrap();
    let mut mw = mw.unwrap();
    mw.set_iterator_index(1).unwrap();
    let input = ImageResource::MagickWand(mw);

    let id = identify_ping(&input).unwrap();
    assert_eq!(100, id.resolution.width);
    assert_eq!(4, id.number_of_frames);
    let mw = input.as_magick_wand().unwrap();
    assert_eq!(1, mw.get_iterator_index());
    assert!(mw.next_image());
    assert_eq!(2, mw.get_iterator_index());

    mw.reset_iterator();
    identify_ping(&input).unwrap();
    assert!(mw.next_image());
    assert_eq!(0, mw.get_iterator_index());

    while mw.next_image() {}
    let expected = mw.clone();
    expected.reset_iterator();
    while expected.next_image() {}
    identify_ping(&input).unwrap();
    assert_eq!(expected.next_image(), mw.next_image());
    assert_eq!(expected.get_iterator_index(), mw.get_iterator_index());
}

#[test]
fn to_png_keeps_the_first_png_frame() {
    start_call_once();
    let mut source = MagickWand::new();
    for (size, color) in [(16, "red"), (32, "blue")] {
        let mut background = PixelWand::new();
        background.set_color(color).unwrap();
        let mut frame = MagickWand::new();
        frame.new_image(size, size, &background).unwrap();
        frame.set_image_format("PNG").unwrap();
        source.add_image(&frame).unwrap();
    }
    let input = ImageResource::MagickWand(source);
    let mut output = ImageResource::Data(Vec::new());
    to_png(&mut output, &input, &PNGConfig::new()).unwrap();

    let mut mw = None;
    let id = identify_read(&mut mw, &output).unwrap();
    assert_eq!(16, id.resolution.width);
    assert_eq!(16, id.resolution.height);
    assert_eq!(
        [255, 0, 0, 255],
        mw.unwrap().export_image_pixels(0, 0, 1, 1, "RGBA").unwrap().as_slice()
    );
}

#[test]
fn failed_animation_encoding_keeps_output_data() {
    start_call_once();
    let mut source = MagickWand::new();
    for color in ["red", "blue"] {
        let mut background = PixelWand::new();
        background.set_color(color).unwrap();
        let mut frame = MagickWand::new();
        // WebP cannot encode a width greater than 16383 pixels.
        frame.new_image(16384, 1, &background).unwrap();
        frame.set_image_format("TIFF").unwrap();
        source.add_image(&frame).unwrap();
    }
    let input = ImageResource::MagickWand(source);
    let mut output = ImageResource::Data(b"old output".to_vec());

    assert!(to_webp(&mut output, &input, &WEBPConfig::new()).is_err());
    assert_eq!(b"old output", output.as_u8_slice().unwrap());
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

    let mut config = GIFConfig::new();
    config.width = 100;
    config.shrink_only = false;

    // The requested canvas size is unchanged, so smaller patches must not be enlarged.
    to_gif(&mut output, &input, &config).unwrap();

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
fn read_apng_keeps_animation_timing() {
    for input in [
        ImageResource::from_path(INPUT_TIMED_APNG_IMAGE_PATH),
        ImageResource::Data(fs::read(INPUT_TIMED_APNG_IMAGE_PATH).unwrap()),
    ] {
        let mut mw = None;
        let id = identify_read(&mut mw, &input).unwrap();

        assert_eq!("PNG", id.format);
        assert_eq!(4, id.number_of_frames);
        assert!(!id.has_unreadable_frames);

        let mw = mw.unwrap();
        mw.reset_iterator();
        for delay in [10, 20, 30, 40] {
            assert!(mw.next_image());
            assert_eq!(100, mw.get_image_width());
            assert_eq!(60, mw.get_image_height());
            assert_eq!(delay, mw.get_image_delay());
            assert_eq!(3, mw.get_image_iterations());
        }
    }
}

#[test]
fn apng_keeps_metadata() {
    start_call_once();
    let path = "tests/data/animation_metadata.png";
    let original = MagickWand::new();
    original.read_image(path).unwrap();
    let icc = original.write_image_blob("ICC").unwrap();
    let exif = original.write_image_blob("EXIF").unwrap();

    for input in [ImageResource::from_path(path), ImageResource::Data(fs::read(path).unwrap())] {
        let mut mw = None;
        let id = identify_read(&mut mw, &input).unwrap();
        assert_eq!(4, id.number_of_frames);
        assert!(!id.has_unreadable_frames);
        let mw = mw.unwrap();
        mw.reset_iterator();
        for delay in [10, 20, 30, 40] {
            assert!(mw.next_image());
            assert_eq!(delay, mw.get_image_delay());
            assert_eq!(3, mw.get_image_iterations());
            assert_eq!("APNG metadata test", mw.get_image_property("Comment").unwrap());
            let frame = MagickWand::new_from_image(&mw.get_image().unwrap()).unwrap();
            assert_eq!(icc, frame.write_image_blob("ICC").unwrap());
            assert_eq!(exif, frame.write_image_blob("EXIF").unwrap());
        }

        let input = ImageResource::MagickWand(mw);
        for strip_metadata in [false, true] {
            let mut output = ImageResource::Data(Vec::new());
            let mut config = TIFFConfig::new();
            config.strip_metadata = strip_metadata;
            to_tiff(&mut output, &input, &config).unwrap();
            let mut mw = None;
            identify_read(&mut mw, &output).unwrap();
            let mw = mw.unwrap();
            mw.reset_iterator();
            while mw.next_image() {
                let frame = MagickWand::new_from_image(&mw.get_image().unwrap()).unwrap();
                if strip_metadata {
                    assert!(frame.write_image_blob("ICC").is_err());
                } else {
                    assert_eq!(icc, frame.write_image_blob("ICC").unwrap());
                }
            }
        }
    }
}

#[test]
fn apng_to_gif_and_webp() {
    let input = ImageResource::Data(fs::read(INPUT_TIMED_APNG_IMAGE_PATH).unwrap());
    let mut gif = ImageResource::Data(Vec::new());
    let mut webp = ImageResource::from_path("tests/data/animation_apng_output.webp");

    to_gif(&mut gif, &input, &GIFConfig::new()).unwrap();

    let mut config = WEBPConfig::new();
    config.quality = 100;
    to_webp(&mut webp, &input, &config).unwrap();

    assert_eq!("GIF", identify_ping(&gif).unwrap().format);
    assert_eq!("WEBP", identify_ping(&webp).unwrap().format);
    assert_same_animation(&input, &gif);
    assert_same_animation(&input, &webp);
}

#[test]
fn webp_and_gif_keep_transparent_overlays() {
    start_call_once();

    let mut red = PixelWand::new();
    red.set_color("red").unwrap();
    let mut source = MagickWand::new();
    source.new_image(4, 2, &red).unwrap();
    source.set_image_delay(10).unwrap();
    source.set_image_iterations(3).unwrap();
    source.set_image_dispose(DisposeType::None).unwrap();
    source.set_image_format("GIF").unwrap();

    let mut transparent = PixelWand::new();
    transparent.set_color("none").unwrap();
    let mut overlay = MagickWand::new();
    overlay.new_image(4, 2, &transparent).unwrap();
    overlay.import_image_pixels(3, 0, 1, 1, &[0, 0, 255, 255], "RGBA").unwrap();
    overlay.set_image_delay(30).unwrap();
    overlay.set_image_dispose(DisposeType::None).unwrap();
    overlay.set_image_format("GIF").unwrap();
    source.add_image(&overlay).unwrap();

    let input = ImageResource::MagickWand(source);
    let mut webp = ImageResource::Data(Vec::new());
    let mut config = WEBPConfig::new();
    config.quality = 100;
    to_webp(&mut webp, &input, &config).unwrap();
    assert_same_animation(&input, &webp);

    let mut gif = ImageResource::Data(Vec::new());
    to_gif(&mut gif, &webp, &GIFConfig::new()).unwrap();
    assert_same_animation(&input, &gif);

    let mut encoded_again = ImageResource::Data(Vec::new());
    to_webp(&mut encoded_again, &webp, &config).unwrap();
    assert_same_animation(&input, &encoded_again);
}

fn assert_same_animation(expected: &ImageResource, actual: &ImageResource) {
    let mut expected_mw = None;
    let mut actual_mw = None;
    identify_read(&mut expected_mw, expected).unwrap();
    identify_read(&mut actual_mw, actual).unwrap();
    let expected = expected_mw.unwrap().coalesce().unwrap();
    let actual = actual_mw.unwrap().coalesce().unwrap();

    assert_eq!(expected.get_number_images(), actual.get_number_images());
    expected.reset_iterator();
    actual.reset_iterator();
    assert_eq!(expected.get_image_iterations(), actual.get_image_iterations());
    while expected.next_image() {
        assert!(actual.next_image());
        assert_eq!(expected.get_image_delay(), actual.get_image_delay());
        let width = expected.get_image_width();
        let height = expected.get_image_height();
        assert_eq!(width, actual.get_image_width());
        assert_eq!(height, actual.get_image_height());
        let expected_pixels = expected.export_image_pixels(0, 0, width, height, "RGBA").unwrap();
        let actual_pixels = actual.export_image_pixels(0, 0, width, height, "RGBA").unwrap();
        for (expected, actual) in expected_pixels.chunks_exact(4).zip(actual_pixels.chunks_exact(4))
        {
            assert_eq!(expected[3], actual[3]);
            // Fully transparent pixels can have different hidden RGB values after encoding.
            if expected[3] != 0 {
                assert_eq!(expected, actual);
            }
        }
    }
}

#[test]
fn to_webp_crops_and_resizes_every_frame() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);
    let mut output = ImageResource::Data(Vec::new());
    let mut config = WEBPConfig::new();
    config.crop = Some(Crop::Center(1.0, 1.0));
    config.width = 30;
    to_webp(&mut output, &input, &config).unwrap();

    let mut mw = None;
    identify_read(&mut mw, &output).unwrap();
    let mw = mw.unwrap().coalesce().unwrap();
    assert_eq!(4, mw.get_number_images());
    mw.reset_iterator();
    while mw.next_image() {
        assert_eq!(30, mw.get_image_width());
        assert_eq!(30, mw.get_image_height());
        assert_eq!(20, mw.get_image_delay());
    }
}

#[test]
fn to_tiff_keeps_complete_animation_frames() {
    let input = ImageResource::from_path(INPUT_IMAGE_PATH);
    let mut output = ImageResource::Data(Vec::new());
    to_tiff(&mut output, &input, &TIFFConfig::new()).unwrap();

    let mut mw = None;
    identify_read(&mut mw, &output).unwrap();
    let mw = mw.unwrap();
    assert_eq!(4, mw.get_number_images());
    mw.reset_iterator();
    while mw.next_image() {
        assert_eq!(100, mw.get_image_width());
        assert_eq!(60, mw.get_image_height());
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
fn to_png_keeps_the_first_frame_canvas() {
    start_call_once();
    let mut color = PixelWand::new();
    color.set_color("red").unwrap();
    let mut mw = MagickWand::new();
    mw.new_image(2, 2, &color).unwrap();
    mw.reset_image_page("8x4+2+1").unwrap();
    mw.set_image_format("GIF").unwrap();
    let input = ImageResource::MagickWand(mw);
    let mut output = ImageResource::Data(Vec::new());
    to_png(&mut output, &input, &PNGConfig::new()).unwrap();

    let id = identify_ping(&output).unwrap();
    assert_eq!(8, id.resolution.width);
    assert_eq!(4, id.resolution.height);
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
