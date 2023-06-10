#![feature(never_type)]
#![feature(decl_macro)]
#![feature(try_blocks)]

extern crate gen_minirust;

pub use gen_minirust::lang::*;
pub use gen_minirust::mem::*;
pub use gen_minirust::prelude::NdResult;
pub use gen_minirust::prelude::*;

pub use gen_minirust::libspecr::hidden::*;
pub use gen_minirust::libspecr::prelude::*;
pub use gen_minirust::libspecr::*;

pub use std::format;
pub use std::result::Result;
pub use std::string::String;

pub mod build;
pub mod fmt;
pub mod run;
pub mod mock_write;
