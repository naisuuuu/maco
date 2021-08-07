//! Test conversion outputs.

use std::path::PathBuf;

use image::open;
use imageproc::assert_pixels_eq;
use maco::{convert, ConvertParams};

const BASE_PATH: [&str; 3] = [".", "tests", "images"];

#[test]
fn convert_sample() {
    let path: PathBuf = BASE_PATH.iter().collect();

    let want = open(&path.join("wikipe-tan-want.png"))
        .unwrap()
        .into_luma8();

    // TODO: For some reason reading the non-grayscale image and converting to grayscale produces a
    // different result than converting to grayscale using python's pillow (current test baseline).
    // This needs some more investigating. Ideally, we want to open "wikipe-tan.png" here instead.
    let got = open(&path.join("wikipe-tan-grayscale.png"))
        .unwrap()
        .into_luma8();

    let params = ConvertParams::builder().build();
    let got = convert(got, &params);

    assert_pixels_eq!(got, want);
}
