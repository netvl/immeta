extern crate immeta;

use immeta::{Dimensions, BaseMetadata};

#[test]
fn test_jpeg() {
    let md = immeta::load_from_file("tests/images/owlet.jpg").unwrap();

    assert_eq!(md.dimensions(), Dimensions::from((1280, 857)));
}

