use image_convert::{
    BMPConfig, ImageResource, JPGConfig, MagickError, PNGConfig, TIFFConfig, WEBPConfig,
    identify_ping, identify_read,
    magick_rust::{MagickWand, OrientationType, PixelWand, ResolutionType},
    start_call_once, to_bmp, to_jpg, to_png, to_tiff, to_webp,
};

const PPI: (f64, f64) = (300.0, 200.0);

fn source(color: &str) -> MagickWand {
    start_call_once();
    let mut background = PixelWand::new();
    background.set_color(color).unwrap();
    let mut image = MagickWand::new();
    image.new_image(16, 16, &background).unwrap();
    image.set_image_format("TIFF").unwrap();
    image.set_image_resolution(100.0, 50.0).unwrap();
    image.set_image_units(ResolutionType::PixelsPerCentimeter).unwrap();
    image.set_image_property("comment", "source metadata marker").unwrap();
    image.profile_image("icc", include_bytes!("../src/srgb.icc").as_slice()).unwrap();
    image
}

fn convert(
    format: &str,
    output: &mut ImageResource,
    input: &ImageResource,
    strip_metadata: bool,
    ppi: Option<(f64, f64)>,
) -> Result<(), MagickError> {
    macro_rules! config {
        ($config:ident) => {
            $config {
                strip_metadata,
                ppi,
                sharpen: 0.0,
                ..$config::new()
            }
        };
    }
    match format {
        "BMP" => to_bmp(output, input, &config!(BMPConfig)),
        "JPEG" => to_jpg(output, input, &config!(JPGConfig)),
        "PNG" => to_png(output, input, &config!(PNGConfig)),
        "TIFF" => to_tiff(output, input, &config!(TIFFConfig)),
        "WEBP" => to_webp(output, input, &config!(WEBPConfig)),
        _ => unreachable!(),
    }
}

fn assert_ppi(expected: (f64, f64), actual: (f64, f64)) {
    // PNG and BMP store integer pixels per meter, so conversion back to inches is approximate.
    assert!((expected.0 - actual.0).abs() < 0.026, "Expected {expected:?}, got {actual:?}");
    assert!((expected.1 - actual.1).abs() < 0.026, "Expected {expected:?}, got {actual:?}");
}

fn png_chunks(data: &[u8]) -> Vec<(&[u8], &[u8])> {
    assert_eq!(b"\x89PNG\r\n\x1A\n", &data[..8]);
    let mut chunks = Vec::new();
    let mut offset = 8;
    while offset < data.len() {
        let length = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        chunks.push((&data[offset + 4..offset + 8], &data[offset + 8..offset + 8 + length]));
        offset += 12 + length;
    }
    chunks
}

#[test]
fn explicit_ppi_survives_stripping_in_every_density_format() {
    let directory = tempfile::tempdir().unwrap();
    let image = source("rgba(180,100,50,0.5)");
    let data = image.write_image_blob("TIFF").unwrap();
    let path = directory.path().join("input.tif");
    std::fs::write(&path, &data).unwrap();
    let inputs = [
        ImageResource::MagickWand(image),
        ImageResource::Data(data),
        ImageResource::from_path(path),
    ];

    for input in &inputs {
        for strip_metadata in [false, true] {
            for format in ["BMP", "JPEG", "PNG", "TIFF", "WEBP"] {
                let path = directory.path().join(format!("output.{}", format.to_lowercase()));
                let outputs = [
                    ImageResource::Data(Vec::new()),
                    ImageResource::from_path(path),
                    ImageResource::MagickWand(MagickWand::new()),
                ];
                for mut output in outputs {
                    convert(format, &mut output, input, strip_metadata, Some(PPI)).unwrap();
                    // PNG density is written only when encoding, so encode a wand output before checking it.
                    let data = match output {
                        ImageResource::Data(data) => data,
                        ImageResource::Path(path) => std::fs::read(path).unwrap(),
                        ImageResource::MagickWand(image) => image.write_image_blob(format).unwrap(),
                    };
                    if format == "PNG" {
                        let chunks = png_chunks(&data);
                        let density = chunks.iter().find(|(name, _)| *name == b"pHYs").unwrap().1;
                        assert_eq!(11811, u32::from_be_bytes(density[..4].try_into().unwrap()));
                        assert_eq!(7874, u32::from_be_bytes(density[4..8].try_into().unwrap()));
                        assert_eq!(1, density[8]);
                        if strip_metadata {
                            assert!(!chunks.iter().any(|(name, _)| {
                                matches!(*name, b"iCCP" | b"eXIf" | b"tEXt" | b"iTXt" | b"zTXt")
                            }));
                        }
                    }
                    let output = ImageResource::Data(data);
                    assert_ppi(PPI, identify_ping(&output).unwrap().ppi);
                    let mut decoded = None;
                    assert_ppi(PPI, identify_read(&mut decoded, &output).unwrap().ppi);
                    let decoded = decoded.unwrap();
                    if strip_metadata {
                        assert!(decoded.write_image_blob("ICC").is_err());
                        assert!(decoded.get_image_property("comment").is_err());
                    }
                    if format == "PNG" {
                        assert_eq!(
                            [180, 100, 50, 128],
                            decoded.export_image_pixels(0, 0, 1, 1, "RGBA").unwrap().as_slice()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn no_ppi_override_keeps_each_formats_source_density_behavior() {
    let input = ImageResource::MagickWand(source("red"));
    for strip_metadata in [false, true] {
        for format in ["BMP", "JPEG", "PNG", "TIFF", "WEBP"] {
            let mut output = ImageResource::Data(Vec::new());
            convert(format, &mut output, &input, strip_metadata, None).unwrap();
            let expected = if format == "WEBP" || (format == "PNG" && strip_metadata) {
                (0.0, 0.0)
            } else {
                (254.0, 127.0)
            };
            assert_ppi(expected, identify_ping(&output).unwrap().ppi);
            assert_ppi(expected, identify_read(&mut None, &output).unwrap().ppi);
        }
    }
}

// An EXIF fixture with a camera make and a nested capture date, with optional existing density.
fn exif(big_endian: bool, density: bool) -> Vec<u8> {
    let short = |value: u16| {
        if big_endian { value.to_be_bytes() } else { value.to_le_bytes() }
    };
    let long = |value: u32| {
        if big_endian { value.to_be_bytes() } else { value.to_le_bytes() }
    };
    let make = b"camera marker\0";
    let date = b"2026:10:10 12:34:56\0";
    let count = if density { 5 } else { 2 };
    let make_offset = 8 + 6 + u32::from(count) * 12;
    let nested_offset = make_offset + make.len() as u32;
    let date_offset = nested_offset + 18;
    let rational_offset = date_offset + date.len() as u32;
    let mut profile = b"Exif\0\0".to_vec();
    profile.extend_from_slice(if big_endian { b"MM" } else { b"II" });
    profile.extend_from_slice(&short(42));
    profile.extend_from_slice(&long(8));
    profile.extend_from_slice(&short(count));
    let entry = |tag, format, count, value: [u8; 4]| {
        [short(tag).as_slice(), &short(format), &long(count), &value].concat()
    };
    profile.extend(entry(0x010F, 2, make.len() as u32, long(make_offset)));
    if density {
        profile.extend(entry(0x011A, 5, 1, long(rational_offset)));
        profile.extend(entry(0x011B, 5, 1, long(rational_offset + 8)));
        let unit = short(2);
        profile.extend(entry(0x0128, 3, 1, [unit[0], unit[1], 0, 0]));
    }
    profile.extend(entry(0x8769, 4, 1, long(nested_offset)));
    profile.extend_from_slice(&long(0));
    profile.extend_from_slice(make);
    profile.extend_from_slice(&short(1));
    profile.extend(entry(0x9003, 2, date.len() as u32, long(date_offset)));
    profile.extend_from_slice(&long(0));
    profile.extend_from_slice(date);
    if density {
        for value in [72, 96] {
            profile.extend_from_slice(&long(value));
            profile.extend_from_slice(&long(1));
        }
    }
    profile
}

#[test]
fn webp_ppi_keeps_other_exif_only_when_metadata_is_kept() {
    for big_endian in [false, true] {
        for density in [false, true] {
            let mut image = source("red");
            image.profile_image("exif", exif(big_endian, density).as_slice()).unwrap();
            image.set_image_resolution(72.0, 96.0).unwrap();
            image.set_image_units(ResolutionType::PixelsPerInch).unwrap();
            let original = image.write_image_blob("EXIF").unwrap();
            let input = ImageResource::MagickWand(image);
            for strip_metadata in [false, true] {
                let mut output = ImageResource::Data(Vec::new());
                convert("WEBP", &mut output, &input, strip_metadata, Some(PPI)).unwrap();
                let mut decoded = None;
                assert_ppi(PPI, identify_read(&mut decoded, &output).unwrap().ppi);
                let decoded = decoded.unwrap();
                for (key, expected) in [
                    ("exif:Make", "camera marker"),
                    ("exif:DateTimeOriginal", "2026:10:10 12:34:56"),
                ] {
                    if strip_metadata {
                        assert!(decoded.get_image_property(key).is_err());
                    } else {
                        assert_eq!(expected, decoded.get_image_property(key).unwrap());
                    }
                }
                assert_eq!("300/1", decoded.get_image_property("exif:XResolution").unwrap());
                assert_eq!("200/1", decoded.get_image_property("exif:YResolution").unwrap());
                assert_eq!("2", decoded.get_image_property("exif:ResolutionUnit").unwrap());
            }
            assert_eq!(original, input.as_magick_wand().unwrap().write_image_blob("EXIF").unwrap());
        }
    }
}

#[test]
fn webp_ping_reads_exif_density_in_centimeters_from_data_and_paths() {
    let image = source("red");
    image.profile_image("exif", exif(true, true).as_slice()).unwrap();
    let input = ImageResource::MagickWand(image);
    let mut output = ImageResource::Data(Vec::new());
    convert("WEBP", &mut output, &input, false, None).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("image.webp");
    std::fs::write(&path, output.as_u8_slice().unwrap()).unwrap();
    for input in [
        output,
        ImageResource::from_path(&path),
        ImageResource::Path(format!("WEBP:{}", path.display())),
    ] {
        assert_ppi((254.0, 127.0), identify_ping(&input).unwrap().ppi);
        assert_ppi((254.0, 127.0), identify_read(&mut None, &input).unwrap().ppi);
    }
}

#[test]
fn webp_ppi_rounds_exif_resolution() {
    let input = ImageResource::MagickWand(source("red"));
    let mut output = ImageResource::Data(Vec::new());
    convert("WEBP", &mut output, &input, true, Some((300.25, 200.75))).unwrap();
    assert_ppi((300.0, 201.0), identify_ping(&output).unwrap().ppi);
    assert_ppi((300.0, 201.0), identify_read(&mut None, &output).unwrap().ppi);
}

#[test]
fn invalid_exif_directory_returns_an_error_unless_stripped() {
    for profile in [
        b"Exif\0\0II*\0".as_slice(),
        b"Exif\0\0II*\0\xFF\xFF\xFF\xFF".as_slice(),
        b"Exif\0\0II*\0\x08\0\0\0\xFF\xFF\0\0\0\0".as_slice(),
    ] {
        let image = source("red");
        image.profile_image("exif", profile).unwrap();
        let input = ImageResource::MagickWand(image);
        let mut output = ImageResource::Data(b"original output".to_vec());
        assert!(convert("WEBP", &mut output, &input, false, Some(PPI)).is_err());
        assert_eq!(b"original output", output.as_u8_slice().unwrap());
        convert("WEBP", &mut output, &input, true, Some(PPI)).unwrap();
        assert_ppi(PPI, identify_ping(&output).unwrap().ppi);
    }
}

#[test]
fn webp_ping_handles_odd_unknown_chunks() {
    let input = ImageResource::MagickWand(source("red"));
    let mut output = ImageResource::Data(Vec::new());
    convert("WEBP", &mut output, &input, true, Some(PPI)).unwrap();
    let mut data = output.into_vec().unwrap();
    let offset = data.windows(4).position(|name| name == b"EXIF").unwrap();
    // An odd-sized RIFF chunk is followed by a padding byte.
    data.splice(offset..offset, *b"JUNK\x01\0\0\0\x07\0");
    let size = (data.len() - 8) as u32;
    data[4..8].copy_from_slice(&size.to_le_bytes());
    let output = ImageResource::Data(data);
    assert_ppi(PPI, identify_ping(&output).unwrap().ppi);
    assert_ppi(PPI, identify_read(&mut None, &output).unwrap().ppi);
}

#[test]
fn explicit_ppi_survives_stripping_on_tiff_pages_and_webp_animation() {
    let mut images = MagickWand::new();
    for (color, delay) in [("red", 10), ("blue", 20)] {
        let mut frame = source(color);
        frame.set_image_delay(delay).unwrap();
        images.add_image(&frame).unwrap();
    }
    let input = ImageResource::MagickWand(images);
    let directory = tempfile::tempdir().unwrap();
    for strip_metadata in [false, true] {
        for format in ["TIFF", "WEBP"] {
            let path = directory.path().join(format!("animation.{}", format.to_lowercase()));
            for mut output in [ImageResource::Data(Vec::new()), ImageResource::from_path(path)] {
                convert(format, &mut output, &input, strip_metadata, Some(PPI)).unwrap();
                assert_ppi(PPI, identify_ping(&output).unwrap().ppi);
                let mut decoded = None;
                assert_eq!(2, identify_read(&mut decoded, &output).unwrap().number_of_frames);
                let decoded = decoded.unwrap();
                decoded.reset_iterator();
                while decoded.next_image() {
                    assert_ppi(PPI, decoded.get_image_resolution().unwrap());
                    assert_eq!(ResolutionType::PixelsPerInch, decoded.get_image_units());
                }
            }
        }
    }
}

#[test]
fn restored_webp_exif_preserves_orientation_for_conversion() {
    let mut original = None;
    identify_read(&mut original, &ImageResource::from_path("tests/data/orientation.jpg")).unwrap();
    let exif = original.unwrap().write_image_blob("EXIF").unwrap();
    let mut frames = MagickWand::new();
    for color in ["red", "blue"] {
        let mut frame = source(color);
        frame.crop_image(16, 8, 0, 0).unwrap();
        frame.reset_image_page("").unwrap();
        frame.profile_image("exif", exif.as_slice()).unwrap();
        frame.set_image_orientation(OrientationType::RightTop).unwrap();
        frame.set_image_delay(10).unwrap();
        frames.add_image(&frame).unwrap();
    }
    let input = ImageResource::MagickWand(frames);
    let mut webp = ImageResource::Data(Vec::new());
    to_webp(&mut webp, &input, &WEBPConfig {
        strip_metadata: false,
        respect_orientation: false,
        ppi: Some(PPI),
        ..WEBPConfig::new()
    })
    .unwrap();
    let mut decoded = None;
    identify_read(&mut decoded, &webp).unwrap();
    let decoded = decoded.unwrap();
    decoded.reset_iterator();
    while decoded.next_image() {
        assert_eq!(OrientationType::RightTop, decoded.get_image_orientation());
    }
    let mut png = ImageResource::Data(Vec::new());
    to_png(&mut png, &webp, &PNGConfig {
        ppi: Some(PPI),
        ..PNGConfig::new()
    })
    .unwrap();
    let id = identify_ping(&png).unwrap();
    assert_eq!(8, id.resolution.width);
    assert_eq!(16, id.resolution.height);
    assert_ppi(PPI, id.ppi);
}
