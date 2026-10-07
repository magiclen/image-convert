use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use magick_rust::MagickWand;

/// The resource of an image. It can be an input resource or an output resource.
#[derive(Debug)]
pub enum ImageResource {
    /// A path of an image file.
    Path(String),
    /// The data of an image.
    /// SVG data can start with a UTF-8 BOM, whitespace, comments, an XML declaration or a document type declaration.
    Data(Vec<u8>),
    /// A `MagickWand` instance which holds an image.
    MagickWand(MagickWand),
}

impl ImageResource {
    /// Create an image resource from a path.
    ///
    /// The path is stored as a UTF-8 string, so a path which is not valid UTF-8 is converted lossily.
    pub fn from_path<P: AsRef<Path>>(path: P) -> ImageResource {
        ImageResource::Path(path.as_ref().to_string_lossy().into_owned())
    }

    /// Create an image resource from a reader.
    pub fn from_reader<R: Read>(mut reader: R) -> Result<ImageResource, io::Error> {
        let mut buffer = Vec::new();

        reader.read_to_end(&mut buffer)?;

        Ok(ImageResource::Data(buffer))
    }

    /// Create an empty image resource with a specific capacity.
    ///
    /// The capacity is not reused when this resource is used as an output resource, because the output data is a newly allocated vec.
    pub fn with_capacity(capacity: usize) -> ImageResource {
        ImageResource::Data(Vec::with_capacity(capacity))
    }
}

impl ImageResource {
    /// Convert this `ImageResource` instance into a path string (if it is possible).
    pub fn into_string(self) -> Option<String> {
        if let ImageResource::Path(p) = self { Some(p) } else { None }
    }

    /// Convert this `ImageResource` instance into a path buffer (if it is possible).
    pub fn into_path_buf(self) -> Option<PathBuf> {
        if let ImageResource::Path(p) = self { Some(PathBuf::from(p)) } else { None }
    }

    /// Convert this `ImageResource` instance into a data vec (if it is possible).
    pub fn into_vec(self) -> Option<Vec<u8>> {
        if let ImageResource::Data(d) = self { Some(d) } else { None }
    }

    /// Convert this `ImageResource` instance into a `MagickWand` (if it is possible).
    pub fn into_magick_wand(self) -> Option<MagickWand> {
        if let ImageResource::MagickWand(mw) = self { Some(mw) } else { None }
    }
}

impl ImageResource {
    /// Convert this `ImageResource` instance into a path string slice (if it is possible).
    pub fn as_str(&self) -> Option<&str> {
        if let ImageResource::Path(p) = self { Some(p.as_str()) } else { None }
    }

    /// Convert this `ImageResource` instance into a path (if it is possible).
    pub fn as_path(&self) -> Option<&Path> {
        if let ImageResource::Path(p) = self { Some(p.as_ref()) } else { None }
    }

    /// Convert this `ImageResource` instance into a data slice (if it is possible).
    pub fn as_u8_slice(&self) -> Option<&[u8]> {
        if let ImageResource::Data(d) = self { Some(d.as_slice()) } else { None }
    }

    /// Convert this `ImageResource` instance into a `MagickWand` reference (if it is possible).
    pub fn as_magick_wand(&self) -> Option<&MagickWand> {
        if let ImageResource::MagickWand(mw) = self { Some(mw) } else { None }
    }
}
