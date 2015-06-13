extern crate immeta;

use immeta::{Dimensions, Metadata, MetadataBox};
use immeta::formats::jpeg::JpegMetadata;
use immeta::formats::png::{self, PngMetadata};

const OWLET_DIM: Dimensions = Dimensions {
    width: 1280,
    height: 857
};

#[test]
fn test_jpeg() {
    let md = immeta::load_from_file("tests/images/owlet.jpg").unwrap();

    assert_eq!(md.mime_type(), "image/jpeg");
    assert_eq!(md.dimensions(), OWLET_DIM);
    assert_eq!(md.color_depth(), None);

    let md = md.downcast::<JpegMetadata>().ok().expect("not JPEG metadata");
    assert_eq!(md.dimensions, OWLET_DIM);
}

#[test]
fn test_png() {
    let md = immeta::load_from_file("tests/images/owlet.png").unwrap();

    assert_eq!(md.mime_type(), "image/png");
    assert_eq!(md.dimensions(), OWLET_DIM);
    assert_eq!(md.color_depth(), Some(24));

    let md = md.downcast::<PngMetadata>().ok().expect("not PNG metadata");
    assert_eq!(md.dimensions, OWLET_DIM);
    assert_eq!(md.color_type, png::ColorType::Rgb);
    assert_eq!(md.color_depth, 24);
    assert_eq!(md.compression_method, png::CompressionMethod::DeflateInflate);
    assert_eq!(md.filter_method, png::FilterMethod::AdaptiveFiltering);
    assert_eq!(md.interlace_method, png::InterlaceMethod::Disabled);
}

