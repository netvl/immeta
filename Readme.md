immeta, an image metadata inspection library in Rust
====================================================

**Unmaintained: feel free to fork.**

---

[![Build Status][travis]](https://travis-ci.org/netvl/immeta) [![crates.io][crates]](https://crates.io/crates/immeta)

  [travis]: https://img.shields.io/travis/netvl/immeta.svg?style=flat-square
  [crates]: https://img.shields.io/crates/v/immeta.svg?style=flat-square

[Documentation](https://docs.rs/immeta/)

immeta is an image metadata processing library. It allows you to inspect metadata, that is,
image dimensions, color information, etc. of various image formats.

Currently the following image formats are supported:
 * JPEG
 * PNG 1.2
 * GIF (87a and 89a)
 * WEBP

Support for more will come in future versions.

**Important note:** this library is not intended to load actual image contents, i.e. the pixel
data. If you need this functionality, consider using other libraries like [image](https://crates.io/crates/image).

## Usage

Just add a dependency to your `Cargo.toml`:

```toml
[dependencies]
immeta = "0.4"
```

You can see an example on how to use it in `tests/test.rs`.


## Changelog

### Version 0.4.1

* Updated arrayvec dependency to 0.5.

### Version 0.4.0

* Updated num-traits dependency to 0.2.

### Version 0.3.6

* Updated arrayvec dependency to 0.4.

### Version 0.3.5

* Updated byteorder dependency to 1.0.

### Version 0.3.4

* Improved JPEG format parser, it now should be panic-free.

### Version 0.3.3

* Switched to num_traits dependency from just num. num_traits is only used in a limited way,
  so this doesn't seem to be a breaking change.

### Version 0.3.2

* Bumped byteorder dependency to 0.5.

### Version 0.3.1

* Now WEBP parser does not panic on yet unsupported VP8 chunk types but returns an error instead.

### Version 0.3.0

* Added unknown disposal method for GIF format. This is a breaking change.

### Version 0.2.4

* Fixed GIF local color table parsing.

### Version 0.2.3

* Bumped byteorder dependency up to 0.4.

### Version 0.2.2

* Added missing `Debug`, `Clone`, `Eq` and `PartialEq` implementations for `GenericMetadata`.

### Version 0.2.1

* Added `std::error::Error` implementation for `immeta::Error` to facilitate interoperation
  with other code.

### Version 0.2.0

* Added basic support for WEBP format, along with RIFF utils.
* Improved API.

### Version 0.1.0

* Initial release, support for GIF, PNG and JPEG.

## License

This library is licensed under MIT license.


---
Copyright (c) Vladimir Matveev, 2015
