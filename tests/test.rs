extern crate immeta;

use immeta::{Dimensions, Metadata};

#[test]
fn test_jpeg() {
    let md = immeta::load_from_file("tests/images/owlet.jpg").unwrap();

    assert_eq!(md.mime_type(), "image/jpeg");
    assert_eq!(md.dimensions(), Dimensions::from((1280, 857)));
    assert_eq!(md.bit_depth(), None);
}

