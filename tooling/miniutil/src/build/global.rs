use crate::build::*;

//bullshit
pub fn global_int<T: TypeConv>() -> Global {
    let bytes = List::from_elem(Some(0), T::get_size().bytes());
    
    Global { 
        bytes, 
        relocations: list!(), 
        align: T::get_align(), 
    }
}