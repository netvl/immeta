immeta, an image metadata inspection library in Rust
====================================================

[![Build Status](https://travis-ci.org/netvl/immeta.svg?branch=master)](https://travis-ci.org/netvl/immeta) [![crates.io](https://img.shields.io/crates/v/immeta.svg)](https://crates.io/crates/immeta)

[Documentation](https://netvl.github.io/immeta/)

immeta is an image metadata processing library. It allows you to inspect metadata, that is, image dimensions, color information, etc. of various image formats.

Currently the following image formats are supported:
 * JPEG
 * PNG 1.2
 * GIF (87a and 89a)
 * WEBP

Support for more will come in future versions.

**Important note:** this library is not intended to load actual image contents, i.e. the pixel data. If you need this functionality, consider using other libraries like [image](https://crates.io/crates/image).

## Usage

Just add a dependency in your `Cargo.toml`:

```toml
[dependencies]
immeta = "0.2"
```

You can see an example on how to use it in `tests/test.rs`.


## Changelog

### Version 0.2.0

* Added basic support for WEBP format, along with RIFF utils
* Improved API

### Version 0.1.0

* Initial release, support for GIF, PNG and JPEG

## License

This library is licensed under MIT license.


---
Copyright (c) Vladimir Matveev, 2015
