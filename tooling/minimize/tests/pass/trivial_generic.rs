fn trivial_generic<T>(_t: T) {}

fn main() {
    trivial_generic::<()>(());
}

