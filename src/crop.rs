/// How to crop an image.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Crop {
    /// CenterCrop at a fixed ratio, which is made up of a width and a height.
    Center(f64, f64),
}
