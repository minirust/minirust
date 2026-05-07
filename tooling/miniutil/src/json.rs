use std::{
    io::{BufReader, Read},
    ops::Deref,
};

use minirust_rs::lang::Program;

pub fn dump_program(prog: &Program) {
    serde_json::to_writer(std::io::stdout(), &prog).expect("Failed to format program as JSON!")
}

#[track_caller]
pub fn assert_roundtrip(prog: &Program) {
    let Ok(deser) = serde_json::to_vec(prog) else {
        panic!("Could not deserialize program while asserting that it round-trips!")
    };
    let Ok(reser) = serde_json::from_slice(deser.deref()) else {
        panic!("Could not serialize program again while asserting that it round-trips!")
    };
    assert_eq!(*prog, reser)
}

pub fn load_program(r: impl Read) -> Program {
    serde_json::from_reader(BufReader::new(r)).expect("Failed to parse JSON into program!")
}
