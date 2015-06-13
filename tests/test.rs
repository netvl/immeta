extern crate immeta;

use immeta::{Dimensions, Metadata, MetadataBox};
use immeta::formats::{png, gif, jpeg};

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

    let md = md.downcast::<jpeg::Metadata>().ok().expect("not JPEG metadata");
    assert_eq!(md.dimensions, OWLET_DIM);
}

#[test]
fn test_png() {
    let md = immeta::load_from_file("tests/images/owlet.png").unwrap();

    assert_eq!(md.mime_type(), "image/png");
    assert_eq!(md.dimensions(), OWLET_DIM);
    assert_eq!(md.color_depth(), Some(24));

    let md = md.downcast::<png::Metadata>().ok().expect("not PNG metadata");
    assert_eq!(md.dimensions, OWLET_DIM);
    assert_eq!(md.color_type, png::ColorType::Rgb);
    assert_eq!(md.color_depth, 24);
    assert_eq!(md.compression_method, png::CompressionMethod::DeflateInflate);
    assert_eq!(md.filter_method, png::FilterMethod::AdaptiveFiltering);
    assert_eq!(md.interlace_method, png::InterlaceMethod::Disabled);
}


#[test]
fn test_gif_plain() {
    let md = immeta::load_from_file("tests/images/owlet.gif").unwrap();

    assert_eq!(md.mime_type(), "image/gif");
    assert_eq!(md.dimensions(), OWLET_DIM);
    assert_eq!(md.color_depth(), None);

    let md = md.downcast::<gif::Metadata>().ok().expect("not GIF metadata");
    assert_eq!(md.version, gif::Version::V89a);
    assert_eq!(md.dimensions, OWLET_DIM);
    assert_eq!(md.global_color_table, true);
    assert_eq!(md.global_color_table_sorted, false);
    assert_eq!(md.global_color_table_size, 256);
    assert_eq!(md.color_resolution, 8);
    assert_eq!(md.background_color_index, 0);
    assert_eq!(md.pixel_aspect_ratio, 0);
    assert_eq!(md.blocks, vec![
        gif::Block::GraphicControlExtension(gif::GraphicControlExtension {
            disposal_method: gif::DisposalMethod::None,
            user_input: false,
            transparent_color: false,
            transparent_color_index: 0,
            delay_time: 0
        }),
        gif::Block::ApplicationExtension(gif::ApplicationExtension {
            application_identifier: *b"ImageMag",
            authentication_code: *b"ick"
        }),
        gif::Block::ImageDescriptor(gif::ImageDescriptor {
            left: 0, top: 0,
            width: 1280, height: 857,
            local_color_table: false,
            local_color_table_sorted: false,
            local_color_table_size: 0,
            interlace: false
        })
    ])
}
