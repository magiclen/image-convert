use crate::magick_rust::bindings;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Ordinalize)]
#[repr(isize)]
pub enum InterlaceType {
    UndefinedInterlace = bindings::InterlaceType_UndefinedInterlace as isize,
    NoInterlace        = bindings::InterlaceType_NoInterlace as isize,
    LineInterlace      = bindings::InterlaceType_LineInterlace as isize,
    PlaneInterlace     = bindings::InterlaceType_PlaneInterlace as isize,
    PartitionInterlace = bindings::InterlaceType_PartitionInterlace as isize,
    GIFInterlace       = bindings::InterlaceType_GIFInterlace as isize,
    JPEGInterlace      = bindings::InterlaceType_JPEGInterlace as isize,
    PNGInterlace       = bindings::InterlaceType_PNGInterlace as isize,
}
