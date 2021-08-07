use image::imageops::{resize, FilterType};
use image::{GrayImage, Luma};
use imageproc::contrast::stretch_contrast_mut;
use imageproc::stats::percentile;

/// Converts an image according to given params.
///
/// See also: [`ConvertParams`].
///
/// # Examples
///
/// ```
/// use maco::{convert, ConvertParams};
/// use imageproc::{assert_pixels_eq, gray_image};
///
/// let img = gray_image!(
///     1, 2, 3;
///     5, 6, 7);
///
/// let params = ConvertParams::default();
/// let img = convert(img, &params);
///
/// assert_pixels_eq!(
///     img,
///     gray_image!(
///         0,   23,  58;
///         148, 199, 255)
/// );
/// ```
pub fn convert(image: GrayImage, params: &ConvertParams) -> GrayImage {
    let (width, height) =
        resize_dimensions(image.width(), image.height(), params.width, params.height);
    // If width didn't change, height didn't change either.
    // If width increased but we don't want to upscale, we can skip.
    let mut image = if width == image.width() || (width > image.width() && !params.upscale) {
        image
    } else {
        resize(&image, width, height, params.filter)
    };

    let lower = percentile(&image, params.cutoff);
    let upper = percentile(&image, 100_u8 - params.cutoff);
    // If lower is 0 and upper 255, the histogram won't change, making computation redundant.
    if upper > lower && !(lower == 0 && upper == 255) {
        stretch_contrast_mut(&mut image, lower, upper);
    }

    // If gamma == 1 the image doesn't change.
    if (params.gamma - 1_f64).abs() > 0.001 {
        apply_lut(&mut image, &params.gamma_lut);
    }

    image
}

/// Defines parameters for the [`convert()`] function.
///
/// See also: [`ConvertParamsBuilder`], [`ConvertParamsBuilder::default()`].
///
/// Examples
///
/// ```
/// use maco::ConvertParams;
///
/// ConvertParams::builder().gamma(0.8).cutoff(10_u8).build();
/// ```
#[derive(Debug)]
pub struct ConvertParams {
    width: u32,
    height: u32,
    upscale: bool,
    cutoff: u8,
    filter: FilterType,
    gamma: f64,
    gamma_lut: [u8; 256],
}

impl Default for ConvertParams {
    /// Convenience method wrapping [`ConvertParamsBuilder::default()`].
    fn default() -> Self {
        ConvertParamsBuilder::default().build()
    }
}

impl ConvertParams {
    /// Returns a new [`ConvertParamsBuilder`] with default settings.
    pub fn builder() -> ConvertParamsBuilder {
        ConvertParamsBuilder::default()
    }
}

/// Builds [`ConvertParams`].
pub struct ConvertParamsBuilder {
    width: u32,
    height: u32,
    upscale: bool,
    cutoff: u8,
    filter: FilterType,
    gamma: f64,
}

impl Default for ConvertParamsBuilder {
    /// Default values are equal to the following:
    ///
    /// ```ignore
    /// ConvertParamsBuilder {
    ///     width: 1920,
    ///     height: 1920,
    ///     upscale: false,
    ///     cutoff: 1,
    ///     filter: FilterType::CatmullRom,
    ///     gamma: 0.75,
    /// }
    /// ```
    fn default() -> Self {
        ConvertParamsBuilder {
            width: 1920,
            height: 1920,
            upscale: false,
            cutoff: 1,
            filter: FilterType::CatmullRom,
            gamma: 0.75,
        }
    }
}

impl ConvertParamsBuilder {
    /// Sets maximum width for the output image. Aspect ratio will be preserved.
    pub fn width(&mut self, width: u32) -> &mut Self {
        self.width = width;
        self
    }

    /// Sets maximum height for the output image. Aspect ratio will be preserved.
    pub fn height(&mut self, height: u32) -> &mut Self {
        self.height = height;
        self
    }

    /// Sets whether or not an image smaller than the desired width x height should be upscaled.
    pub fn upscale(&mut self, upscale: bool) -> &mut Self {
        self.upscale = upscale;
        self
    }

    /// Sets % of the brightest and darkest pixels to ignore when equalizing the histogram.
    pub fn cutoff(&mut self, upscale_percentile: u8) -> &mut Self {
        self.cutoff = upscale_percentile;
        self
    }

    /// Sets resampling filter used when resizing the image.
    pub fn filter(&mut self, filter: FilterType) -> &mut Self {
        self.filter = filter;
        self
    }

    /// Sets a gamma modifier. Values < 1 darken the image, values > 1 brighten it.
    pub fn gamma(&mut self, gamma: f64) -> &mut Self {
        self.gamma = gamma;
        self
    }

    /// Builds and returns a [`ConvertParams`] instance.
    pub fn build(&self) -> ConvertParams {
        ConvertParams {
            width: self.width,
            height: self.height,
            upscale: self.upscale,
            cutoff: self.cutoff,
            filter: self.filter,
            gamma: self.gamma,
            gamma_lut: generate_gamma_lut(self.gamma),
        }
    }
}

/// Generates a lookup table with gamma modifications applied.
fn generate_gamma_lut(gamma: f64) -> [u8; 256] {
    let mut lut = [0; 256];
    for (i, x) in lut.iter_mut().enumerate() {
        *x = clamp((i as f64 / 255_f64).powf(1_f64 / gamma) * 255_f64)
    }
    lut
}

fn clamp(i: f64) -> u8 {
    if i > 255_f64 {
        return 255;
    }
    if i > 0_f64 {
        return i as u8;
    }
    0
}

/// Applies a lookup table to a grayscale image, i.e. for each pixel, given pixel value x, replaces
/// said pixel with lut[x].
fn apply_lut(image: &mut GrayImage, lut: &[u8; 256]) {
    for p in image.pixels_mut() {
        *p = Luma([lut[p[0] as usize]]);
    }
}

/// Calculates the width and height an image should be resized to.
/// Preserves aspect ratio so that both dimensions are contained within the given `nx` and `ny`.
/// If `nx` or `ny` are 0, their value will by replaced by `x` or `y` respectively, allowing for
/// easier downscaling to desired size in one dimension.
fn resize_dimensions(x: u32, y: u32, nx: u32, ny: u32) -> (u32, u32) {
    let nx = if nx > 0 { nx } else { x };
    let ny = if ny > 0 { ny } else { y };

    let ratio = u64::from(x) * u64::from(ny);
    let nratio = u64::from(nx) * u64::from(y);

    let use_y = nratio <= ratio;

    let intermediate = if use_y {
        u64::from(y) * u64::from(nx) / u64::from(x)
    } else {
        u64::from(x) * u64::from(ny) / u64::from(y)
    };
    let intermediate = std::cmp::max(1, intermediate);

    if use_y {
        (nx, intermediate as u32)
    } else {
        (intermediate as u32, ny)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! dimensions_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (x, y, nx, ny, expected) = $value;
                assert_eq!(expected, resize_dimensions(x, y, nx, ny));
            }
        )*
        }
    }

    dimensions_tests! {
        resize_dimensions_x_gt_y: (100, 100, 70, 50, (50, 50)),
        resize_dimensions_y_gt_x: (100, 100, 50, 70, (50, 50)),
        resize_dimensions_0nx: (100, 100, 0, 50, (50, 50)),
        resize_dimensions_0ny: (100, 100, 50, 0, (50, 50)),
    }
}
