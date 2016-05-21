extern crate immeta;
#[macro_use(crate_version)]
extern crate clap;

use std::io::{self, Write};

use clap::{App, AppSettings};

use immeta::GenericMetadata;
use immeta::formats::{jpeg, gif, png, webp};

fn main() {
    let matches = App::new("immeta image analyzer")
        .version(crate_version!())
        .author("Vladimir Matveev <vladimir.matweev@gmail.com>")
        .about("Loads and displays metadata from image files.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .setting(AppSettings::ColoredHelp)
        .args_from_usage(
            "<FILE>  'Input file name'"
        )
        .get_matches();
    
    let file_name = matches.value_of("FILE").unwrap();

    let metadata = match immeta::load_from_file(file_name) {
        Ok(md) => md,
        Err(e) => {
            let _ = writeln!(&mut io::stderr(), "Cannot load image metadata from {}: {}", file_name, e);
            return;
        }
    };

    match metadata {
        GenericMetadata::Jpeg(md) => print_jpeg_metadata(md),
        GenericMetadata::Gif(md) => print_gif_metadata(md),
        GenericMetadata::Png(md) => print_png_metadata(md),
        GenericMetadata::Webp(md) => print_webp_metadata(md),
    }
}

fn print_jpeg_metadata(md: jpeg::Metadata) {
    println!("JPEG image:");
    println!("  Width: {}", md.dimensions.width);
    println!("  Height: {}", md.dimensions.height);
    println!("  Sample precision: {}", md.sample_precision);
    println!("  Baseline: {}", md.baseline);
    println!("  Differential: {}", md.differential);
    println!("  Entropy coding: {}", md.entropy_coding);
    println!("  Coding process: {}", md.coding_process);
}

fn print_gif_metadata(md: gif::Metadata) {
    println!("GIF image:");
    // TODO
}

fn print_png_metadata(md: png::Metadata) {
    println!("PNG image:");
    println!("  Width: {}", md.dimensions.width);
    println!("  Height: {}", md.dimensions.height);
    println!("  Color type: {}", md.color_type);
    println!("  Color depth: {} bpp", md.color_depth);
    println!("  Compression method: {}", md.compression_method);
    println!("  Filter method: {}", md.filter_method);
    println!("  Interlace method: {}", md.interlace_method);
}

fn print_webp_metadata(md: webp::Metadata) {
    println!("WEBP image:");
    // TODO
}
