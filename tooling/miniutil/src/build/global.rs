use crate::build::*;

/// Global Int initialized to zero.
pub fn global_int<T: TypeConv>() -> Global {
    let bytes = List::from_elem(Some(0), T::get_size().bytes());

    Global {
        bytes,
        relocations: list!(),
        align: T::get_align(),
    }
}

/// Global pointer
pub fn global_ptr<T: TypeConv>() -> Global {
    let bytes = List::from_elem(Some(0), <*const T>::get_size().bytes());

    Global {
        bytes,
        relocations: list!(),
        align: <*const T>::get_align(),
    }
}
