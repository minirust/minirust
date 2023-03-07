#![feature(never_type)]

extern crate gen_minirust;

pub use gen_minirust::lang::*;
pub use gen_minirust::mem::*;
pub use gen_minirust::prelude::*;

pub use gen_minirust::libspecr::*;
pub use gen_minirust::libspecr::prelude::*;
pub use gen_minirust::libspecr::hidden::*;

pub use std::format;
pub use std::string::String;
pub use gen_minirust::prelude::NdResult;

pub mod fmt;
pub mod build;
pub mod run;
