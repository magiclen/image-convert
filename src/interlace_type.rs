use crate::magick_rust::bindings;

create_ordinalized_enum!(pub InterlaceType,
    UndefinedInterlace = bindings::InterlaceType_UndefinedInterlace as isize,
    NoInterlace = bindings::InterlaceType_NoInterlace as isize,
    LineInterlace = bindings::InterlaceType_LineInterlace as isize,
    PlaneInterlace = bindings::InterlaceType_PlaneInterlace as isize,
    PartitionInterlace = bindings::InterlaceType_PartitionInterlace as isize,
    GIFInterlace = bindings::InterlaceType_GIFInterlace as isize,
    JPEGInterlace = bindings::InterlaceType_JPEGInterlace as isize,
    PNGInterlace = bindings::InterlaceType_PNGInterlace as isize,
);