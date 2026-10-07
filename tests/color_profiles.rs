use std::io::Cursor;

use image_convert::{
    BMPConfig, Color, GIFConfig, GrayRawConfig, ICOConfig, ImageResource, JPGConfig, MagickError,
    PGMConfig, PNGConfig, TIFFConfig, WEBPConfig, identify_read,
    magick_rust::{MagickWand, PixelWand},
    start_call_once, to_bmp, to_gif, to_gray_raw, to_ico, to_jpg, to_pgm, to_png, to_tiff, to_webp,
};

// These profiles were made with Little CMS 2 using the standard primaries and transfer curves.
const DISPLAY_P3: &[u8] = include_bytes!("data/display_p3.icc");
const ADOBE_RGB: &[u8] = include_bytes!("data/adobe_rgb.icc");
const SRGB: &[u8] = include_bytes!("../src/srgb.icc");
const SOURCE_PIXEL: [u8; 4] = [180, 100, 50, 255];
const PROFILES: [(&[u8], [u8; 4], u8); 2] =
    [(DISPLAY_P3, [193, 95, 34, 255], 111), (ADOBE_RGB, [203, 100, 42, 255], 118)];

fn profiled_image(profile: &[u8], rgba: [u8; 4]) -> MagickWand {
    start_call_once();
    let [r, g, b, a] = rgba;
    let mut color = PixelWand::new();
    color.set_color(Color::Rgba(r, g, b, a).to_magick_color().as_ref()).unwrap();
    let mut image = MagickWand::new();
    image.new_image(16, 16, &color).unwrap();
    image.set_image_depth(8).unwrap();
    image.set_image_format("PNG").unwrap();
    image.profile_image("icc", profile).unwrap();
    image
}

fn read_image(resource: &ImageResource) -> MagickWand {
    let mut image = None;
    identify_read(&mut image, resource).unwrap();
    image.unwrap()
}

fn pixel(image: &MagickWand) -> [u8; 4] {
    image.export_image_pixels(0, 0, 1, 1, "RGBA").unwrap().try_into().unwrap()
}

fn icc_profile(image: &MagickWand) -> Result<Vec<u8>, MagickError> {
    MagickWand::new_from_image(&image.get_image()?).unwrap().write_image_blob("ICC")
}

fn assert_pixel(expected: [u8; 4], actual: [u8; 4], tolerance: u8) {
    assert_eq!(expected[3], actual[3]);
    for (expected, actual) in expected[..3].iter().zip(&actual[..3]) {
        assert!(expected.abs_diff(*actual) <= tolerance, "Expected {expected}, got {actual}.");
    }
}

#[test]
fn stripped_profiles_convert_colors() {
    type Convert = fn(&mut ImageResource, &ImageResource) -> Result<(), MagickError>;
    let conversions: [Convert; 6] = [
        |output, input| to_bmp(output, input, &BMPConfig::new()),
        |output, input| to_png(output, input, &PNGConfig::new()),
        |output, input| to_gif(output, input, &GIFConfig::new()),
        |output, input| to_tiff(output, input, &TIFFConfig::new()),
        |output, input| {
            to_jpg(output, input, &JPGConfig {
                quality: Some(100),
                force_to_chroma_quartered: false,
                ..JPGConfig::new()
            })
        },
        |output, input| {
            to_webp(output, input, &WEBPConfig {
                quality: 100,
                ..WEBPConfig::new()
            })
        },
    ];

    for (profile, expected, _) in PROFILES {
        let original = profiled_image(profile, SOURCE_PIXEL);
        let data = original.write_image_blob("PNG").unwrap();
        for input in [ImageResource::MagickWand(original), ImageResource::Data(data)] {
            for (format, convert) in
                ["BMP", "PNG", "GIF", "TIFF", "JPEG", "WEBP"].into_iter().zip(conversions)
            {
                let mut output = ImageResource::Data(Vec::new());
                convert(&mut output, &input).unwrap();
                let output = read_image(&output);
                let actual = pixel(&output);
                assert_eq!(expected[3], actual[3]);
                assert!(
                    expected[..3]
                        .iter()
                        .zip(&actual[..3])
                        .all(|(expected, actual)| expected.abs_diff(*actual) <= 2),
                    "{format}: expected {expected:?}, got {actual:?}."
                );
                assert!(icc_profile(&output).is_err());
            }
            if let Some(original) = input.as_magick_wand() {
                assert_eq!(SOURCE_PIXEL, pixel(original));
                assert_eq!(profile, icc_profile(original).unwrap());
                assert_eq!("PNG", original.get_image_format().unwrap());
            }
        }
    }
}

#[test]
fn profiled_jpeg_path_and_data_convert_colors() {
    for (profile, expected, _) in PROFILES {
        let mut source = profiled_image(profile, SOURCE_PIXEL);
        source.set_image_compression_quality(100).unwrap();
        source.set_sampling_factors(&[1.0, 1.0, 1.0]).unwrap();
        let data = source.write_image_blob("JPEG").unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("image.jpg");
        std::fs::write(&path, &data).unwrap();

        for input in [ImageResource::from_path(&path), ImageResource::Data(data)] {
            let mut output = ImageResource::Data(Vec::new());
            to_png(&mut output, &input, &PNGConfig::new()).unwrap();
            assert_pixel(expected, pixel(&read_image(&output)), 2);
        }
    }
}

// Put an ICC profile into JPEG data without checking it, like a broken encoder would.
fn jpeg_with_icc_profile(jpeg: &[u8], profile: &[u8]) -> Vec<u8> {
    let length = (profile.len() + 16) as u16;

    [
        &jpeg[..2],
        &[0xFF, 0xE2],
        &length.to_be_bytes(),
        b"ICC_PROFILE\0",
        &[1, 1],
        profile,
        &jpeg[2..],
    ]
    .concat()
}

#[test]
fn unusable_profiles_keep_pixels() {
    let mut source = profiled_image(DISPLAY_P3, SOURCE_PIXEL);
    source.strip_image().unwrap();
    source.set_image_compression_quality(100).unwrap();
    source.set_sampling_factors(&[1.0, 1.0, 1.0]).unwrap();
    let jpeg = source.write_image_blob("JPEG").unwrap();
    let expected = pixel(&read_image(&ImageResource::Data(jpeg.clone())));

    // A profile for gray pixels cannot describe these RGB pixels.
    let mut gray = DISPLAY_P3.to_vec();
    gray[16..20].copy_from_slice(b"GRAY");

    let mut short_size = DISPLAY_P3.to_vec();
    short_size[..4].copy_from_slice(&128u32.to_be_bytes());
    let mut large_count = DISPLAY_P3.to_vec();
    large_count[128..132].copy_from_slice(&65535u32.to_be_bytes());
    let mut invalid_offset = DISPLAY_P3.to_vec();
    invalid_offset[136..140].copy_from_slice(&(DISPLAY_P3.len() as u32).to_be_bytes());
    let mut invalid_length = DISPLAY_P3.to_vec();
    invalid_length[140..144].copy_from_slice(&u32::MAX.to_be_bytes());

    for profile in [
        &[0x55; 200],
        &DISPLAY_P3[..300],
        gray.as_slice(),
        short_size.as_slice(),
        large_count.as_slice(),
        invalid_offset.as_slice(),
        invalid_length.as_slice(),
    ] {
        let input = ImageResource::Data(jpeg_with_icc_profile(&jpeg, profile));
        let mut output = ImageResource::Data(Vec::new());

        to_png(&mut output, &input, &PNGConfig::new()).unwrap();

        assert_eq!(expected, pixel(&read_image(&output)));
    }
}

// The output is a wand, not PNG data, because encoding a 12-megapixel PNG takes seconds.
// `stripped_profiles_convert_colors` and `retained_profiles_keep_pixels` cover the PNG encoder.
fn assert_heic_color_profile(
    path: &str,
    profile_description: &str,
    resolution: (usize, usize),
    samples: [(isize, isize, [u8; 4]); 3],
) {
    let (original_profile, original_pixels) = {
        let original = read_image(&ImageResource::from_path(path));
        assert_eq!("HEIC", original.get_image_format().unwrap());
        assert_eq!(profile_description, original.get_image_property("icc:description").unwrap());
        let pixels: [[u8; 4]; 3] = samples.map(|(x, y, _)| {
            original.export_image_pixels(x, y, 1, 1, "RGBA").unwrap().try_into().unwrap()
        });
        (icc_profile(&original).unwrap(), pixels)
    };

    for input in [ImageResource::from_path(path), ImageResource::Data(std::fs::read(path).unwrap())]
    {
        for strip_metadata in [true, false] {
            let mut output = ImageResource::MagickWand(MagickWand::new());
            let mut config = PNGConfig::new();
            config.strip_metadata = strip_metadata;

            to_png(&mut output, &input, &config).unwrap();

            let output = output.into_magick_wand().unwrap();
            assert_eq!("PNG", output.get_image_format().unwrap());
            assert_eq!(resolution.0, output.get_image_width());
            assert_eq!(resolution.1, output.get_image_height());
            if strip_metadata {
                assert!(icc_profile(&output).is_err());
            } else {
                assert_eq!(original_profile, icc_profile(&output).unwrap());
            }
            for ((x, y, expected), original) in samples.into_iter().zip(original_pixels) {
                let actual =
                    output.export_image_pixels(x, y, 1, 1, "RGBA").unwrap().try_into().unwrap();
                if strip_metadata {
                    assert_pixel(expected, actual, 3);
                } else {
                    assert_eq!(original, actual);
                }
            }
        }
    }
}

// The expected sample colors were computed with Little CMS 2 outside this crate.
#[test]
fn display_p3_heic_keeps_or_converts_its_color_profile() {
    assert_heic_color_profile("tests/data/display_p3.heic", "Display P3", (4032, 3024), [
        (1408, 304, [113, 160, 214, 255]),
        (304, 912, [168, 202, 229, 255]),
        (304, 2544, [121, 128, 78, 255]),
    ]);
}

#[test]
fn dci_p3_srgb_eotf_heic_keeps_or_converts_its_color_profile() {
    assert_heic_color_profile(
        "tests/data/dci_p3_srgb_eotf.heic",
        "sRGB EOTF with DCI-P3 Color Gamut",
        (4096, 2304),
        [
            (720, 336, [112, 155, 0, 255]),
            (1408, 2080, [58, 96, 0, 255]),
            (3904, 2272, [161, 206, 97, 255]),
        ],
    );
}

#[test]
fn retained_profiles_keep_pixels() {
    for (profile, ..) in PROFILES {
        let input = ImageResource::MagickWand(profiled_image(profile, SOURCE_PIXEL));
        let mut output = ImageResource::Data(Vec::new());
        let mut config = PNGConfig::new();
        config.strip_metadata = false;

        to_png(&mut output, &input, &config).unwrap();

        let output = read_image(&output);
        assert_eq!(SOURCE_PIXEL, pixel(&output));
        assert_eq!(profile, icc_profile(&output).unwrap());
    }
}

#[test]
fn ico_applies_profiles_to_every_size() {
    for (profile, expected, _) in PROFILES {
        let input = ImageResource::MagickWand(profiled_image(profile, SOURCE_PIXEL));
        for strip_metadata in [false, true] {
            let mut output = ImageResource::Data(Vec::new());
            let mut config = ICOConfig::new();
            config.size = vec![(16, 16), (256, 256)];
            config.strip_metadata = strip_metadata;

            to_ico(&mut output, &input, &config).unwrap();

            let icon = ico::IconDir::read(Cursor::new(output.into_vec().unwrap())).unwrap();
            assert_eq!(2, icon.entries().len());
            assert!(!icon.entries()[0].is_png());
            assert!(icon.entries()[1].is_png());
            for entry in icon.entries() {
                let image = entry.decode().unwrap();
                assert_pixel(expected, image.rgba_data()[..4].try_into().unwrap(), 1);
            }
        }
    }
}

#[test]
fn gray_outputs_apply_profiles_when_metadata_is_kept() {
    for (profile, _, expected) in PROFILES {
        let input = ImageResource::MagickWand(profiled_image(profile, SOURCE_PIXEL));
        for strip_metadata in [false, true] {
            let mut output = ImageResource::Data(Vec::new());
            let mut raw_config = GrayRawConfig::new();
            raw_config.strip_metadata = strip_metadata;

            to_gray_raw(&mut output, &input, &raw_config).unwrap();
            assert_eq!(vec![expected; 16 * 16], output.into_vec().unwrap());

            let mut output = ImageResource::Data(Vec::new());
            let mut pgm_config = PGMConfig::new();
            pgm_config.strip_metadata = strip_metadata;

            to_pgm(&mut output, &input, &pgm_config).unwrap();
            assert_pixel([expected, expected, expected, 255], pixel(&read_image(&output)), 1);
        }
    }
}

#[test]
fn background_color_keeps_its_srgb_appearance() {
    for (profile, ..) in PROFILES {
        let input = ImageResource::MagickWand(profiled_image(profile, [180, 100, 50, 0]));
        for strip_metadata in [false, true] {
            let mut output = ImageResource::Data(Vec::new());
            let mut config = TIFFConfig::new();
            config.background_color = Some(Color::Rgb(80, 120, 160));
            config.strip_metadata = strip_metadata;

            to_tiff(&mut output, &input, &config).unwrap();

            let output = read_image(&output);
            if strip_metadata {
                assert!(icc_profile(&output).is_err());
            } else {
                assert_eq!(profile, icc_profile(&output).unwrap());
                output.profile_image("icc", SRGB).unwrap();
            }
            assert_pixel([80, 120, 160, 255], pixel(&output), 1);
        }
    }
}

#[test]
fn tiff_applies_each_profile_and_strips_page_labels() {
    start_call_once();
    let mut source = MagickWand::new();
    let labels = ["First profile page", "Second profile page"];
    for ((profile, ..), label) in PROFILES.into_iter().zip(labels) {
        let mut image = profiled_image(profile, SOURCE_PIXEL);
        image.set_image_format("TIFF").unwrap();
        image.set_image_property("label", label).unwrap();
        source.add_image(&image).unwrap();
    }
    let input = ImageResource::MagickWand(source);

    for strip_metadata in [false, true] {
        let mut config = TIFFConfig::new();
        config.strip_metadata = strip_metadata;
        let mut output = ImageResource::MagickWand(MagickWand::new());
        to_tiff(&mut output, &input, &config).unwrap();
        let output = output.into_magick_wand().unwrap();
        assert_eq!(2, output.get_number_images());
        output.reset_iterator();
        for ((profile, expected, _), label) in PROFILES.into_iter().zip(labels) {
            assert!(output.next_image());
            if strip_metadata {
                assert_pixel(expected, pixel(&output), 1);
                assert!(icc_profile(&output).is_err());
                assert_eq!("", output.get_image_property("label").unwrap());
            } else {
                assert_eq!(SOURCE_PIXEL, pixel(&output));
                assert_eq!(profile, icc_profile(&output).unwrap());
                assert_eq!(label, output.get_image_property("label").unwrap());
            }
        }

        let mut output = ImageResource::Data(Vec::new());
        to_tiff(&mut output, &input, &config).unwrap();
        let data = output.as_u8_slice().unwrap();
        for label in labels {
            assert_eq!(
                !strip_metadata,
                data.windows(label.len()).any(|value| value == label.as_bytes())
            );
        }
        let output = read_image(&output);
        assert_eq!(2, output.get_number_images());
        output.reset_iterator();
        for (profile, expected, _) in PROFILES {
            assert!(output.next_image());
            if strip_metadata {
                assert_pixel(expected, pixel(&output), 1);
            } else {
                assert_eq!(SOURCE_PIXEL, pixel(&output));
                assert_eq!(profile, icc_profile(&output).unwrap());
            }
        }
    }
}
