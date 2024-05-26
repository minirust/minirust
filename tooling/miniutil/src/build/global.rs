use crate::build::*;

impl ProgramBuilder {
    pub fn declare_global_zero_initialized<T: TypeConv>(&mut self) -> PlaceExpr {
        let bytes = List::from_elem(Some(0), T::get_size().bytes());
        let global = Global { bytes, relocations: list!(), align: <T>::get_align() };
        let name = GlobalName(Name::from_internal(self.next_global));
        self.next_global += 1;
        self.globals.try_insert(name, global).unwrap();
        global_by_name::<T>(name)
    }
}

/// Global Int initialized to zero.
pub fn global_int<T: TypeConv>() -> Global {
    let bytes = List::from_elem(Some(0), T::get_size().bytes());

    Global { bytes, relocations: list!(), align: T::get_align() }
}

/// Global pointer
pub fn global_ptr<T: TypeConv>() -> Global {
    let bytes = List::from_elem(Some(0), <*const T>::get_size().bytes());

    Global { bytes, relocations: list!(), align: <*const T>::get_align() }
}
