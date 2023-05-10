use std::{io::Write, str::from_utf8, cell::RefCell, rc::{Rc, Weak}};

#[derive(Default)]
pub struct MockBuffer {
    buffer: Rc<RefCell<Vec<u8>>>,
}

impl MockBuffer {
    pub fn new() -> Self {
        Default::default()
    }

    pub(crate) fn out(&self) -> MockWrite {
        MockWrite { buffer: Rc::downgrade(&self.buffer) }
    }

    pub fn into_strings(self) -> Vec<String> {
        let slice = self.buffer.borrow();

        let string = from_utf8(&slice).unwrap().to_string();

        string.split("\n").map( 
            |str| str.to_string() 
        ).collect()
    }
}

#[derive(Default)]
pub(crate) struct MockWrite {
    buffer: Weak<RefCell<Vec<u8>>>,
}

impl Write for MockWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let Some(rc) = self.buffer.upgrade() else {
            return Ok(0);
        };

        rc.borrow_mut().extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_works() {
        let mock = MockBuffer::new();

        let mut stream = mock.out();
        write!(stream, "{}\n", 1).unwrap();

        let strings = mock.into_strings();

        assert_eq!(strings.len(), 2);

        assert_eq!(strings[0], "1");
    }
}
