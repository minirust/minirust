#![feature(never_type)]
#![feature(decl_macro)]
#![feature(try_blocks)]

extern crate minirust_rs;

pub use minirust_rs::libspecr::hidden::*;
pub use minirust_rs::libspecr::prelude::*;
pub use minirust_rs::libspecr::*;

pub use minirust_rs::lang::*;
pub use minirust_rs::mem::*;
pub use minirust_rs::prelude::*;
pub use minirust_rs::prelude::NdResult;

pub use std::format;
pub use std::result::Result;
pub use std::string::String;

pub mod build;
pub mod fmt;
pub mod run;
pub mod mock_write;

pub type DefaultTarget = x86_64;
