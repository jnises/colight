/// TODO: there is a better standard way of doing this right?
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{Read, Write},
    rc::Rc,
};

struct ReadHalf(Rc<RefCell<VecDeque<u8>>>);

impl Read for ReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().read(buf)
    }
}

struct WriteHalf(Rc<RefCell<VecDeque<u8>>>);

impl Write for WriteHalf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn make_read_writer() -> (ReadHalf, WriteHalf) {
    let buf = Rc::new(RefCell::new(VecDeque::new()));
    (ReadHalf(buf.clone()), WriteHalf(buf))
}

pub(crate) struct AnsiStripReader<R> {
    input: R,
    buf: ReadHalf,
    stripper: strip_ansi_escapes::Writer<WriteHalf>,
}

impl<R> AnsiStripReader<R> {
    pub(crate) fn new(input: R) -> Self {
        let (buf, writer) = make_read_writer();
        let stripper = strip_ansi_escapes::Writer::new(writer);
        Self {
            input,
            buf,
            stripper,
        }
    }
}

impl<R> Read for AnsiStripReader<R>
where
    R: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            match self.buf.read(buf)? {
                0 => {
                    let mut input_buf = [0; 64];
                    match self.input.read(&mut input_buf)? {
                        0 => return Ok(0),
                        n => {
                            self.stripper.write_all(&input_buf[..n])?;
                        }
                    }
                }
                n => return Ok(n),
            }
        }
    }
}
