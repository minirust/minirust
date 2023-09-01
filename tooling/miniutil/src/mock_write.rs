use std::{io::Write, str::from_utf8, cell::RefCell, rc::Rc};

use minirust_rs::libspecr::hidden::GcCompat;

/// A buffer to mock a GcWrite object.
/// It is used to catch output from MiniRust code for testing.
#[derive(Default, Clone)]
pub struct MockWrite {
    buffer: Rc<RefCell<Vec<u8>>>,
}

impl MockWrite {
    pub fn new() -> Self {
        Default::default()
    }

    /// Get all output lines as Strings.
    pub fn into_strings(self) -> Vec<String> {
        let slice = self.buffer.borrow();

        let string = from_utf8(&slice).unwrap().to_string();

        string.lines().map(|s| s.to_string()).collect()
    }
}

impl Write for MockWrite {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buffer.borrow_mut().extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Nothing within has anything to do with specr-lang. This points to nothing.
impl GcCompat for MockWrite {
    fn points_to(&self, _buffer: &mut std::collections::HashSet<usize>) { }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_works() {
        let mock = MockWrite::new();

        let mut stream = mock.clone();
        write!(stream, "{}\n", 1).unwrap();

        let strings = mock.into_strings();

        assert_eq!(strings.len(), 2);

        assert_eq!(strings[0], "1");
    }
}
