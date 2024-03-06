
struct RepeatN {
    val: u8,
    repetitions: u8,
}

impl Iterator for RepeatN {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.repetitions > 0 {
            self.repetitions -= 1;
            Some(self.val)
        } else {
            None
        }
    }
}

fn main() {
    let iter = RepeatN {
        val: 42,
        repetitions: 3,
    };
    let mut sum = 0;
    for i in iter {
        sum += i;
    }
    if sum != 126 {
        unreachable!()
    }
}
