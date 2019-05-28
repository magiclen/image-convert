use std::path::{Path, PathBuf};
use std::io::{self, Read};

use crate::magick_rust::MagickWand;

/// The resource of an image. It can be an input resource or an output resource.
#[derive(Debug)]
pub enum ImageResource {
    Path(String),
    Data(Vec<u8>),
    MagickWand(MagickWand),
}

impl ImageResource {
    /// Create an image resource from a path.
    pub fn from_path<P: AsRef<Path>>(path: P) -> ImageResource {
        ImageResource::Path(path.as_ref().to_string_lossy().to_string())
    }

    /// Create an image resource from a reader.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<ImageResource, io::Error> {
        let mut buffer = Vec::new();

        reader.read_to_end(&mut buffer)?;

        Ok(ImageResource::Data(buffer))
    }

    /// Create an empty image resource with a specific capacity.
    pub fn with_capacity(capacity: usize) -> ImageResource {
        ImageResource::Data(Vec::with_capacity(capacity))
    }
}

impl ImageResource {
    /// Convert this `ImageResource` instance into a path string (if it is possible).
    pub fn into_string(self) -> Option<String> {
        if let ImageResource::Path(p) = self {
            Some(p)
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a path buffer (if it is possible).
    pub fn into_path_buf(self) -> Option<PathBuf> {
        if let ImageResource::Path(p) = self {
            Some(PathBuf::from(p))
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a data vec (if it is possible).
    pub fn into_vec(self) -> Option<Vec<u8>> {
        if let ImageResource::Data(d) = self {
            Some(d)
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a `MagickWand` (if it is possible).
    pub fn into_magick_wand(self) -> Option<MagickWand> {
        if let ImageResource::MagickWand(mw) = self {
            Some(mw)
        } else {
            None
        }
    }
}

impl ImageResource {
    /// Convert this `ImageResource` instance into a path string slice (if it is possible).
    pub fn as_str(&self) -> Option<&str> {
        if let ImageResource::Path(p) = self {
            Some(p.as_str())
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a path (if it is possible).
    pub fn as_path(&self) -> Option<&Path> {
        if let ImageResource::Path(p) = self {
            Some(p.as_ref())
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a data slice (if it is possible).
    pub fn as_u8_slice(&self) -> Option<&[u8]> {
        if let ImageResource::Data(d) = self {
            Some(d.as_slice())
        } else {
            None
        }
    }

    /// Convert this `ImageResource` instance into a `Magickwand` reference (if it is possible).
    pub fn as_magick_wand(&self) -> Option<&MagickWand> {
        if let ImageResource::MagickWand(mw) = self {
            Some(mw)
        } else {
            None
        }
    }
}