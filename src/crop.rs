#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Crop {
    // CenterCrop at a fixed ratio.
    Center(f64, f64),
}
