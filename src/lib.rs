#![feature(core, alloc)]

extern crate byteorder;
extern crate num;

pub use types::{Error, Result, Dimensions, AnimationInfo};
pub use traits::*;
pub use generic::*;

#[macro_use] mod macros;
mod traits;
mod types;
mod utils;
mod generic;

pub mod formats;
