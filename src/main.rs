use std::{cell::RefCell, collections::VecDeque, io::Read};

use colorous::COOL;
use scopeguard::defer;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let so = stdout.lock();
    print_comp(si, so)?;
    Ok(())
}

fn print_comp<I, O>(mut si: I, so: O) -> anyhow::Result<()>
where
    O: WriteColor,
    I: Read,
{
    // refcell so we can reset on scope exit
    let soc = RefCell::new(so);
    defer! {
        soc.borrow_mut().reset().unwrap();
    }
    let pr = |buffered: VecDeque<u8>| -> anyhow::Result<()> {
        if !buffered.is_empty() {
            let compression = 1f32 / buffered.len() as f32;
            let (a, b) = buffered.as_slices();
            for line in a
                .split_inclusive(|&c| c == b'\n')
                .chain(b.split_inclusive(|&c| c == b'\n'))
            {
                // set the color at the start of each line as some terminals seem to reset
                soc.borrow_mut()
                    .set_color(ColorSpec::new().set_fg(Some(color_map(compression))))?;
                soc.borrow_mut().write_all(line)?;
            }
        }
        Ok(())
    };
    const WINDOW_SIZE: usize = 1024;
    let mut searcher = WindowSearcher::new(WINDOW_SIZE);
    loop {
        let mut byte_buf = [0; 1];
        if si.read_exact(&mut byte_buf).is_err() {
            pr(searcher.flush())?;
            break;
        };
        // keep going until we find no more matches
        match searcher.search(byte_buf[0]) {
            SearchState::Buffering => {}
            SearchState::Flushed {
                buffer: last_found_needle,
            } => {
                pr(last_found_needle)?;
            }
        }
    }
    Ok(())
}

fn color_map(t: f32) -> Color {
    let (r, g, b) = COOL.eval_continuous(t.into()).as_tuple();
    Color::Rgb(r, g, b)
}

struct WindowSearcher {
    window_size: usize,
    haystack: VecDeque<u8>,
    needle: VecDeque<u8>,
    matches: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
enum SearchState {
    Buffering,
    Flushed { buffer: VecDeque<u8> },
}

impl WindowSearcher {
    fn new(window_size: usize) -> Self {
        Self {
            window_size,
            haystack: VecDeque::new(),
            needle: VecDeque::new(),
            matches: Vec::new(),
        }
    }

    fn search(&mut self, next_byte: u8) -> SearchState {
        self.matches
            .retain_mut(|i| self.haystack.get(*i + self.needle.len()) == Some(&next_byte));
        let r = if self.matches.is_empty() {
            self.haystack.extend(self.needle.iter());
            while self.haystack.len() > self.window_size {
                self.haystack.pop_front();
            }
            let buffer = std::mem::take(&mut self.needle);
            self.matches = (0..self.haystack.len())
                .filter(|&i| self.haystack[i] == next_byte)
                .collect();
            SearchState::Flushed { buffer }
        } else {
            SearchState::Buffering
        };
        self.needle.push_back(next_byte);
        r
    }

    fn flush(mut self) -> VecDeque<u8> {
        std::mem::take(&mut self.needle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_window_searcher() {
        let mut s = WindowSearcher::new(4);
        assert_eq!(
            s.search(b'a'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"")
            }
        );
        assert_eq!(
            s.search(b'b'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"a")
            }
        );
        assert_eq!(
            s.search(b'a'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"b")
            }
        );
        assert_eq!(s.search(b'b'), SearchState::Buffering);
        assert_eq!(
            s.search(b'c'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"ab")
            }
        );
        assert_eq!(
            s.search(b'a'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"c")
            }
        );
        assert_eq!(
            s.search(b'a'),
            SearchState::Flushed {
                buffer: VecDeque::from(*b"a")
            }
        );
        assert_eq!(s.search(b'b'), SearchState::Buffering);
        assert_eq!(s.flush(), VecDeque::from(*b"ab"));
    }

    struct NullStdout;
    impl WriteColor for NullStdout {
        fn supports_color(&self) -> bool {
            true
        }
        fn set_color(&mut self, _spec: &ColorSpec) -> io::Result<()> {
            Ok(())
        }
        fn reset(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl io::Write for NullStdout {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_print_comp() {
        let mut s = NullStdout;
        print_comp(
            io::Cursor::new(
                b"[Mon Mar 1 09:19:57 CET 2021]path: 
[Mon Mar 1 09:19:57 CET 2021]result of plugin in system Library: 0
[Mon Mar 1 09:19:57 CET 2021]result of plugin in home: 0
[Mon Mar 1 09:20:01 CET 2021] start new app: /Applications/app.app",
            ),
            &mut s,
        )
        .unwrap();
    }

    #[test]
    fn test_print_comp2() {
        let mut s = NullStdout;
        print_comp(
            io::Cursor::new(
                b"ab
ab
ab
ab
",
            ),
            &mut s,
        )
        .unwrap();
    }
}
