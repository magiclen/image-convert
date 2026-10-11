use std::io::{Read, Seek, SeekFrom};

use magick_rust::{MagickError, MagickWand, OrientationType, ResolutionType};

use crate::{
    color_profile::is_usable_profile,
    functions::{for_each_frame, read_image_profile},
};

// The profiles which a WebP container stores for all of its frames.
#[derive(Default)]
pub(crate) struct WebpProfiles {
    icc:  Option<Vec<u8>>,
    exif: Option<Vec<u8>>,
}

// ImageMagick's WebP ping and animated decoder omit container profiles. Read them without decoding pixels.
// The ICC profile is read only if `icc` is `true`.
pub(crate) fn read_webp_profiles(mut reader: impl Read + Seek, icc: bool) -> WebpProfiles {
    let mut profiles = WebpProfiles::default();

    // Like ImageMagick's WebP decoder, ignore invalid data instead of failing to read the image.
    let _ = read_webp_chunks(&mut reader, icc, &mut profiles);

    profiles
}

// Store the profiles found before any invalid data.
fn read_webp_chunks(
    reader: &mut (impl Read + Seek),
    icc: bool,
    profiles: &mut WebpProfiles,
) -> Option<()> {
    let size = reader.seek(SeekFrom::End(0)).ok()?;
    reader.seek(SeekFrom::Start(0)).ok()?;
    let mut header = [0; 12];
    reader.read_exact(&mut header).ok()?;
    if &header[..4] != b"RIFF" || &header[8..] != b"WEBP" {
        return None;
    }
    let mut end = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
    // Like ImageMagick, accept a RIFF size which is 12 or 20 bytes larger than the data, because some writers store it that way.
    if end > size && matches!(end - size, 12 | 20) {
        end = size;
    }
    if end < 12 || end > size {
        return None;
    }
    // Only the extended format starts with VP8X, and ImageMagick reads a profile only when the flags have its bit.
    let mut extended = [0; 9];
    reader.read_exact(&mut extended).ok()?;
    if &extended[..4] != b"VP8X" {
        return None;
    }
    let mut icc = icc && extended[8] & 0x20 != 0;
    let mut exif = extended[8] & 0x08 != 0;
    // `GetMaxProfileSize` was added in ImageMagick 7.1.1-31, so the documentation lists that version as the minimum one.
    // SAFETY: ImageMagick is initialized before reading an image, and this function takes no pointers.
    let max_profile_size = unsafe { magick_rust::bindings::GetMaxProfileSize() };
    let mut offset = 12;
    // The RIFF header and the start of the VP8X chunk have been read.
    let mut position = 21;
    while (icc || exif) && offset < end {
        // Unlike `seek`, `seek_relative` keeps the buffer of a `BufReader`, so skipping small chunks does not read the file again.
        reader.seek_relative(offset as i64 - position as i64).ok()?;
        let mut chunk = [0; 8];
        reader.read_exact(&mut chunk).ok()?;
        position = offset + 8;
        let length = u32::from_le_bytes(chunk[4..].try_into().unwrap());
        offset = position + u64::from(length) + u64::from(length % 2);
        if offset > end {
            return None;
        }
        // Use the first chunk of each profile.
        let profile = match &chunk[..4] {
            b"ICCP" if icc => {
                icc = false;
                &mut profiles.icc
            },
            b"EXIF" if exif => {
                exif = false;
                &mut profiles.exif
            },
            _ => continue,
        };
        // Skip profiles over ImageMagick's policy limit before allocating or reading their data.
        if length as usize > max_profile_size {
            continue;
        }
        let mut data = Vec::new();
        data.try_reserve_exact(length as usize).ok()?;
        reader.by_ref().take(u64::from(length)).read_to_end(&mut data).ok()?;
        position += data.len() as u64;
        *profile = Some(data);
    }
    Some(())
}

// Attach the container profiles to the first frame only, because they belong to the whole image and copying them to every frame would multiply their memory use.
// Conversions use the color profile of the first frame for the frames without one, and `all_frames` gives every frame the orientation and density from EXIF.
pub(crate) fn restore_webp_profiles(
    mw: &mut MagickWand,
    profiles: WebpProfiles,
    all_frames: bool,
) -> Result<(), MagickError> {
    mw.reset_iterator();

    if let Some(icc) = profiles.icc {
        // A still image decoded with webpmux has its profiles already, and attaching an ICC profile again could convert its colors.
        // ImageMagick converts the colors with a device link profile instead of attaching it, so only usable profiles are attached.
        if read_image_profile(mw, "ICC")?.is_none() && is_usable_profile(mw, &icc) {
            attach_profile(mw, "icc", &icc)?;
        }
    }

    let Some(exif) = profiles.exif else {
        return Ok(());
    };

    if read_image_profile(mw, "EXIF")?.is_some() || !attach_profile(mw, "exif", &exif)? {
        return Ok(());
    }

    let attributes = ExifAttributes::read(mw);

    if all_frames {
        for_each_frame(mw, |frame| attributes.apply(frame))
    } else {
        attributes.apply(mw)
    }
}

// Like ImageMagick's decoders, skip a profile which cannot be attached, such as one over the profile size limit of the policy.
fn attach_profile(mw: &mut MagickWand, name: &str, profile: &[u8]) -> Result<bool, MagickError> {
    if mw.profile_image(name, profile).is_ok() {
        return Ok(true);
    }

    mw.clear_exception()?;

    Ok(false)
}

// The attributes which ImageMagick's decoders set from EXIF.
struct ExifAttributes {
    orientation: Option<OrientationType>,
    resolution:  Option<(f64, f64)>,
    units:       Option<ResolutionType>,
}

impl ExifAttributes {
    // Attaching a profile does not initialize the attributes which the normal decoder sets, so read them from the EXIF properties of the current frame.
    fn read(mw: &MagickWand) -> Self {
        let orientation = match mw.get_image_property("exif:Orientation").ok().as_deref() {
            Some("1") => Some(OrientationType::TopLeft),
            Some("2") => Some(OrientationType::TopRight),
            Some("3") => Some(OrientationType::BottomRight),
            Some("4") => Some(OrientationType::BottomLeft),
            Some("5") => Some(OrientationType::LeftTop),
            Some("6") => Some(OrientationType::RightTop),
            Some("7") => Some(OrientationType::RightBottom),
            Some("8") => Some(OrientationType::LeftBottom),
            _ => None,
        };
        let units = match mw.get_image_property("exif:ResolutionUnit").ok().as_deref() {
            Some("1") => Some(ResolutionType::Undefined),
            Some("2") => Some(ResolutionType::PixelsPerInch),
            Some("3") => Some(ResolutionType::PixelsPerCentimeter),
            _ => None,
        };
        let resolution = |key| {
            let value = mw.get_image_property(key).ok()?;
            let (numerator, denominator) = value.split_once('/').unwrap_or((&value, "1"));
            let value =
                numerator.trim().parse::<f64>().ok()? / denominator.trim().parse::<f64>().ok()?;
            (value.is_finite() && value >= 0.0).then_some(value)
        };
        let resolution = match (resolution("exif:XResolution"), resolution("exif:YResolution")) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };

        Self {
            orientation,
            resolution,
            units,
        }
    }

    fn apply(&self, mw: &mut MagickWand) -> Result<(), MagickError> {
        if let Some(orientation) = self.orientation {
            mw.set_image_orientation(orientation)?;
        }
        if let Some((x, y)) = self.resolution {
            mw.set_image_resolution(x, y)?;
            // Like ImageMagick, keep the current units when EXIF has none.
            if let Some(units) = self.units {
                mw.set_image_units(units)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn read_short(self, bytes: &[u8]) -> u16 {
        let bytes = bytes.try_into().unwrap();
        match self {
            Self::Little => u16::from_le_bytes(bytes),
            Self::Big => u16::from_be_bytes(bytes),
        }
    }

    fn read_long(self, bytes: &[u8]) -> u32 {
        let bytes = bytes.try_into().unwrap();
        match self {
            Self::Little => u32::from_le_bytes(bytes),
            Self::Big => u32::from_be_bytes(bytes),
        }
    }

    fn short(self, value: u16) -> [u8; 2] {
        match self {
            Self::Little => value.to_le_bytes(),
            Self::Big => value.to_be_bytes(),
        }
    }

    fn long(self, value: u32) -> [u8; 4] {
        match self {
            Self::Little => value.to_le_bytes(),
            Self::Big => value.to_be_bytes(),
        }
    }
}

// ImageMagick synchronizes existing EXIF density tags when encoding, but does not create them.
// Keep all source data and offsets, including nested directories and thumbnails, when adding tags.
pub(crate) fn prepare_exif_density(mw: &MagickWand) -> Result<(), MagickError> {
    let profile = read_image_profile(mw, "EXIF")?
        .unwrap_or_else(|| b"Exif\0\0II*\0\x08\0\0\0\0\0\0\0\0\0".to_vec());

    let error = || MagickError("Cannot add WebP PPI to an invalid EXIF directory.".into());
    let mut tiff = profile.strip_prefix(b"Exif\0\0").unwrap_or(&profile).to_vec();
    let header = tiff.get(..8).ok_or_else(error)?;
    let order = match &header[..4] {
        b"II*\0" => ByteOrder::Little,
        b"MM\0*" => ByteOrder::Big,
        _ => return Err(error()),
    };
    let offset = order.read_long(&header[4..8]) as usize;
    if offset < 8 {
        return Err(error());
    }
    let directory = tiff.get(offset..).ok_or_else(error)?;
    let count = order.read_short(directory.get(..2).ok_or_else(error)?) as usize;
    let end = 2 + count * 12;
    let next_directory: [u8; 4] =
        directory.get(end..end + 4).ok_or_else(error)?.try_into().unwrap();
    let entries = directory[2..end].chunks_exact(12);

    // Existing tags are updated by ImageMagick, so their directory need not be replaced.
    if [(0x011A, 5), (0x011B, 5), (0x0128, 3)].into_iter().all(|(tag, format)| {
        entries.clone().any(|entry| {
            order.read_short(&entry[..2]) == tag
                && order.read_short(&entry[2..4]) == format
                && order.read_long(&entry[4..8]) == 1
                && (format != 5 || {
                    let offset = order.read_long(&entry[8..]) as usize;
                    offset.checked_add(8).is_some_and(|end| tiff.get(offset..end).is_some())
                })
        })
    }) {
        return Ok(());
    }

    let mut entries: Vec<[u8; 12]> = entries
        .filter(|entry| !matches!(order.read_short(&entry[..2]), 0x011A | 0x011B | 0x0128))
        .map(|entry| entry.try_into().unwrap())
        .collect();
    let count = u16::try_from(entries.len() + 3).map_err(|_| error())?;

    // Append a new first directory so every offset into the original profile remains valid.
    if tiff.len() % 2 != 0 {
        tiff.push(0);
    }
    let offset = u32::try_from(tiff.len()).map_err(|_| error())?;
    let rational_offset = offset.checked_add(6 + u32::from(count) * 12).ok_or_else(error)?;
    rational_offset.checked_add(16).ok_or_else(error)?;

    for (tag, format, value) in [
        (0x011A, 5, order.long(rational_offset)),
        (0x011B, 5, order.long(rational_offset + 8)),
        (0x0128, 3, {
            let unit = order.short(2); // inches
            [unit[0], unit[1], 0, 0]
        }),
    ] {
        let mut entry = [0; 12];
        entry[..2].copy_from_slice(&order.short(tag));
        entry[2..4].copy_from_slice(&order.short(format));
        entry[4..8].copy_from_slice(&order.long(1));
        entry[8..].copy_from_slice(&value);
        entries.push(entry);
    }
    entries.sort_by_key(|entry| order.read_short(&entry[..2]));
    tiff[4..8].copy_from_slice(&order.long(offset));
    tiff.extend_from_slice(&order.short(count));
    for entry in entries {
        tiff.extend_from_slice(&entry);
    }
    tiff.extend_from_slice(&next_directory);
    for _ in 0..2 {
        // Encoding fills in each numerator from the image resolution.
        tiff.extend_from_slice(&order.long(0));
        tiff.extend_from_slice(&order.long(1));
    }

    mw.profile_image("exif", [b"Exif\0\0".as_slice(), &tiff].concat().as_slice())
}
