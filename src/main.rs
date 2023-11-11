mod ansi_stripper;
mod window_searcher;

use std::{cell::RefCell, collections::VecDeque, io::Read};

use clap::Parser;
use colorous::COOL;
use scopeguard::defer;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{ansi_stripper::AnsiStripReader, window_searcher::{WindowSearcher, SearchState}};

/// Highlight characters in a stream based on how compressible they are
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// How long back in history to look for matches
    #[arg(long, default_value_t = 1024)]
    window_size: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let stdin = std::io::stdin();
    let si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let so = stdout.lock();
    print_comp(si, so, args.window_size)?;
    Ok(())
}

fn print_comp<I, O>(si: I, so: O, window_size: usize) -> anyhow::Result<()>
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
    let mut color_stripper = AnsiStripReader::new(si);
    let mut searcher = WindowSearcher::new(window_size);
    loop {
        let mut byte_buf = [0; 1];
        if color_stripper.read_exact(&mut byte_buf).is_err() {
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
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            Ok(buf.len())
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
            1024,
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
            1024
        )
        .unwrap();
    }
}
