extern crate immeta;

use immeta::Dimensions;
use immeta::formats::{png, gif};
use immeta::markers::{Png, Gif, Jpeg};

const OWLET_DIM: Dimensions = Dimensions {
    width: 1280,
    height: 857
};

const DROP_DIM: Dimensions = Dimensions {
    width: 238,
    height: 212
};

#[test]
fn test_jpeg() {
    let md = immeta::load_from_file("tests/images/owlet.jpg").unwrap();

    assert_eq!(md.mime_type(), "image/jpeg");
    assert_eq!(md.dimensions(), OWLET_DIM);

    // let md = Jpeg::from(md).ok()
    let md = md.into::<Jpeg>().ok().expect("not JPEG metadata");
    assert_eq!(md.dimensions, OWLET_DIM);
}

#[test]
fn test_png() {
    let md = immeta::load_from_file("tests/images/owlet.png").unwrap();

    assert_eq!(md.mime_type(), "image/png");
    assert_eq!(md.dimensions(), OWLET_DIM);

    let md = md.into::<Png>().ok().expect("not PNG metadata");
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

    let md = md.into::<Gif>().ok().expect("not GIF metadata");
    assert_eq!(md.version, gif::Version::V89a);
    assert_eq!(md.dimensions, OWLET_DIM);
    assert_eq!(md.global_color_table, Some(gif::ColorTable {
        size: 256,
        sorted: false
    }));
    assert_eq!(md.color_resolution, 256);
    assert_eq!(md.background_color_index, 0);
    assert_eq!(md.pixel_aspect_ratio, 0);
    assert_eq!(md.frames_number(), 1);
    assert_eq!(md.is_animated(), false);
    assert_eq!(md.blocks, vec![
        gif::Block::GraphicControlExtension(gif::GraphicControlExtension {
            disposal_method: gif::DisposalMethod::None,
            user_input: false,
            transparent_color_index: None,
            delay_time: 0
        }),
        gif::Block::ApplicationExtension(gif::ApplicationExtension {
            application_identifier: *b"ImageMag",
            authentication_code: *b"ick"
        }),
        gif::Block::ImageDescriptor(gif::ImageDescriptor {
            left: 0, top: 0,
            width: 1280, height: 857,
            local_color_table: None,
            interlace: false
        })
    ])
}

#[test]
fn test_gif_animated() {
    let md = immeta::load_from_file("tests/images/drop.gif").unwrap();

    assert_eq!(md.mime_type(), "image/gif");
    assert_eq!(md.dimensions(), DROP_DIM);

    let md = md.into::<Gif>().ok().expect("not GIF metadata");
    assert_eq!(md.version, gif::Version::V89a);
    assert_eq!(md.dimensions, DROP_DIM);
    assert_eq!(md.global_color_table, Some(gif::ColorTable {
        size: 256,
        sorted: false
    }));
    assert_eq!(md.color_resolution, 128);
    assert_eq!(md.background_color_index, 255);
    assert_eq!(md.pixel_aspect_ratio, 0);
    assert_eq!(md.frames_number(), 30);
    assert_eq!(md.is_animated(), true);

    let mut blocks = md.blocks.iter();

    assert_eq!(
        blocks.next().unwrap(),
        &gif::Block::ApplicationExtension(gif::ApplicationExtension {
            application_identifier: *b"NETSCAPE",
            authentication_code: *b"2.0"
        })
    );

    assert_eq!(
        blocks.next().unwrap(),
        &gif::Block::CommentExtension(gif::CommentExtension)
    );

    for i in 0..30 {
        match blocks.next() {
            Some(&gif::Block::GraphicControlExtension(ref gce)) => {
                assert_eq!(
                    gce,
                    &gif::GraphicControlExtension {
                        disposal_method: if i == 29 { 
                            gif::DisposalMethod::None 
                        } else { 
                            gif::DisposalMethod::DoNotDispose
                        },
                        user_input: false,
                        transparent_color_index: Some(255),
                        delay_time: 7
                    }
                );
                assert_eq!(gce.delay_time_ms(), 70);
            }
            _ => panic!("Invalid block")
        }
        

        assert_eq!(
            blocks.next().unwrap(),
            &gif::Block::ImageDescriptor(gif::ImageDescriptor {
                left: 0, top: 0,
                width: 238, height: 212,
                local_color_table: None,
                interlace: false
            })
        );
    }

    assert!(blocks.next().is_none());
}
