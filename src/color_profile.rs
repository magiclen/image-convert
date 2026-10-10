use magick_rust::{
    ColorspaceType, MagickError, MagickWand, PixelWand, RenderingIntent, bindings::ExceptionType,
};

use crate::functions::read_image_profile;

// This sRGB profile was made with Little CMS 2 and is free to use.
const SRGB_PROFILE: &[u8] = include_bytes!("srgb.icc");

// Browsers ignore a broken profile, or one made for other color channels, so the pixels keep their values here too.
fn is_usable_profile(mw: &MagickWand, profile: &[u8]) -> bool {
    // The header holds the profile size at 0, the device class at 12, the color space at 16 and the `acsp` signature at 36.
    let Some(header) = profile.get(..128) else {
        return false;
    };

    let size = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;

    if size > profile.len()
        || &header[36..40] != b"acsp"
        || !matches!(&header[12..16], b"mntr" | b"scnr" | b"prtr" | b"spac")
    {
        return false;
    }

    let color_space: &[u8] = match mw.get_image_colorspace() {
        ColorspaceType::sRGB | ColorspaceType::RGB => b"RGB ",
        ColorspaceType::GRAY | ColorspaceType::LinearGRAY => b"GRAY",
        ColorspaceType::CMYK => b"CMYK",
        _ => return false,
    };

    if &header[16..20] != color_space {
        return false;
    }

    let Some(profile) = profile.get(..size).filter(|profile| profile.len() >= 132) else {
        return false;
    };
    let count = u32::from_be_bytes(profile[128..132].try_into().unwrap()) as usize;

    // The tag count and each data range must fit inside the declared profile size.
    if count > (size - 132) / 12 {
        return false;
    }
    let table_end = 132 + count * 12;

    profile[132..table_end].chunks_exact(12).all(|tag| {
        let offset = u32::from_be_bytes(tag[4..8].try_into().unwrap()) as usize;
        let length = u32::from_be_bytes(tag[8..12].try_into().unwrap()) as usize;

        offset >= table_end
            && length >= 8
            && offset.checked_add(length).is_some_and(|end| end <= size)
    })
}

// Read the XYZ values of the red, green and blue colorant tags from the tag table which follows the header.
fn read_colorants(profile: &[u8]) -> Option<[f64; 9]> {
    let count = u32::from_be_bytes(profile.get(128..132)?.try_into().ok()?) as usize;
    let table_end = 132usize.checked_add(count.checked_mul(12)?)?;
    let tags = profile.get(132..table_end)?.chunks_exact(12);

    let mut colorants = [0f64; 9];

    for (signature, values) in [b"rXYZ", b"gXYZ", b"bXYZ"].into_iter().zip(colorants.chunks_mut(3))
    {
        let tag = tags.clone().find(|tag| &tag[..4] == signature)?;
        let offset = u32::from_be_bytes(tag[4..8].try_into().ok()?) as usize;
        let length = u32::from_be_bytes(tag[8..12].try_into().ok()?) as usize;

        if length < 20 {
            return None;
        }

        // An XYZ tag holds its type signature, 4 reserved bytes and 3 s15Fixed16 numbers.
        let data = profile.get(offset..offset.checked_add(20)?)?;

        if &data[..4] != b"XYZ " {
            return None;
        }

        for (value, bytes) in values.iter_mut().zip(data[8..].chunks_exact(4)) {
            *value = f64::from(i32::from_be_bytes(bytes.try_into().ok()?)) / 65536f64;
        }
    }

    Some(colorants)
}

fn has_srgb_colorants(profile: &[u8]) -> bool {
    match (read_colorants(profile), read_colorants(SRGB_PROFILE)) {
        (Some(colorants), Some(srgb)) => {
            colorants.iter().zip(srgb).all(|(value, srgb)| (value - srgb).abs() <= 0.01)
        },
        _ => false,
    }
}

// Convert a grid of colors with the profile, because a profile which keeps them is another variant of sRGB.
fn is_srgb_profile(profile: &[u8], rendering_intent: RenderingIntent) -> Result<bool, MagickError> {
    const STEPS: usize = 9;
    const COUNT: usize = STEPS * STEPS * STEPS;

    if !has_srgb_colorants(profile) {
        return Ok(false);
    }

    let pixels: Vec<u8> = (0..COUNT)
        .flat_map(|index| {
            [index / (STEPS * STEPS), index / STEPS % STEPS, index % STEPS]
                .map(|step| (step * 255 / (STEPS - 1)) as u8)
        })
        .collect();

    let mut probe = MagickWand::new();

    probe.new_image(COUNT, 1, &PixelWand::new())?;
    probe.import_image_pixels(0, 0, COUNT, 1, &pixels, "RGB")?;
    probe.set_image_rendering_intent(rendering_intent)?;

    // The first profile is only attached, and the second one converts the colors.
    apply_icc_profile(&mut probe, profile)?;
    apply_icc_profile(&mut probe, SRGB_PROFILE)?;

    let converted = probe
        .export_image_pixels(0, 0, COUNT, 1, "RGB")
        .ok_or("Cannot check the color profile.")?;

    Ok(converted.iter().zip(&pixels).all(|(converted, pixel)| converted.abs_diff(*pixel) <= 1))
}

/// Remember whether the last checked profile is a variant of sRGB, because the frames of an image usually share one profile.
#[derive(Default)]
pub(crate) struct SrgbCheck {
    last: Option<(Vec<u8>, RenderingIntent, bool)>,
}

impl SrgbCheck {
    // Return the profile of the current frame if its colors have to be converted.
    fn profile_to_convert(&mut self, mw: &MagickWand) -> Result<Option<Vec<u8>>, MagickError> {
        let Some(profile) = read_image_profile(mw, "ICC")? else {
            return Ok(None);
        };

        if profile == SRGB_PROFILE || !is_usable_profile(mw, &profile) {
            return Ok(None);
        }

        let rendering_intent = mw.get_image_rendering_intent();

        let srgb = match &self.last {
            Some((last, last_intent, srgb))
                if *last == profile && *last_intent == rendering_intent =>
            {
                *srgb
            },
            _ => {
                let srgb = is_srgb_profile(&profile, rendering_intent)?;

                self.last = Some((profile.clone(), rendering_intent, srgb));

                srgb
            },
        };

        Ok(if srgb { None } else { Some(profile) })
    }
}

fn apply_icc_profile(mw: &mut MagickWand, profile: &[u8]) -> Result<(), MagickError> {
    mw.clear_exception()?;
    mw.profile_image("icc", profile)?;

    // Without Little CMS, ImageMagick reports success but leaves the colors unchanged.
    if mw.get_exception_type() == ExceptionType::MissingDelegateWarning {
        return Err("ICC color conversion requires ImageMagick with the lcms delegate.".into());
    }

    Ok(())
}

pub(crate) fn convert_to_srgb(
    mw: &mut MagickWand,
    check: &mut SrgbCheck,
) -> Result<(), MagickError> {
    if check.profile_to_convert(mw)?.is_some() {
        apply_icc_profile(mw, SRGB_PROFILE)?;

        // Palette depths describe indices, so use 8-bit channels to keep the converted colors.
        if mw.get_image_depth() < 8 {
            mw.set_image_depth(8)?;
        }
    }

    Ok(())
}

pub(crate) fn convert_background_color(
    mw: &MagickWand,
    color: &PixelWand,
    check: &mut SrgbCheck,
) -> Result<Option<PixelWand>, MagickError> {
    let Some(profile) = check.profile_to_convert(mw)? else {
        return Ok(None);
    };

    let mut background = MagickWand::new();
    background.new_image(1, 1, color)?;
    background.transform_image_colorspace(ColorspaceType::sRGB)?;
    background.set_image_rendering_intent(mw.get_image_rendering_intent())?;
    apply_icc_profile(&mut background, SRGB_PROFILE)?;
    apply_icc_profile(&mut background, &profile)?;

    background
        .get_image_pixel_color(0, 0)
        .map(Some)
        .ok_or_else(|| "Cannot convert the background color to the image color profile.".into())
}
