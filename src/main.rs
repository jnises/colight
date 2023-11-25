mod ansi_stripper;
mod window_searcher;

use std::{cell::RefCell, collections::VecDeque, io::Read};

use clap::Parser;
use colorous::COOL;
use scopeguard::defer;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    ansi_stripper::AnsiStripReader,
    window_searcher::{SearchState, WindowSearcher},
};

/// Highlight characters in a stream based on how compressible they are
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// How long back in history to look for matches
    #[arg(long, default_value_t = 1024)]
    window_size: usize,

    /// How much to penalize age of matches, higher values will make older matches less likely to be highlighted
    #[arg(long, default_value_t = 0.01)]
    age_penalty: f32,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    assert!(args.window_size > 0);
    assert!(args.age_penalty >= 0.0);
    let stdin = std::io::stdin();
    let si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let so = stdout.lock();
    print_comp(si, so, args)?;
    Ok(())
}

fn print_comp<I, O>(si: I, so: O, args: Args) -> anyhow::Result<()>
where
    O: WriteColor,
    I: Read,
{
    let Args {
        window_size,
        age_penalty,
    } = args;
    // refcell so we can reset on scope exit
    let soc = RefCell::new(so);
    defer! {
        soc.borrow_mut().reset().unwrap();
    }
    let pr = |buffered: VecDeque<u8>, age: Option<usize>| -> anyhow::Result<()> {
        if !buffered.is_empty() {
            let score = 1f32 / (buffered.len() as f32 + age.unwrap_or(0) as f32 * age_penalty);
            let (a, b) = buffered.as_slices();
            for line in a
                .split_inclusive(|&c| c == b'\n')
                .chain(b.split_inclusive(|&c| c == b'\n'))
            {
                // set the color at the start of each line as some terminals seem to reset
                soc.borrow_mut()
                    .set_color(ColorSpec::new().set_fg(Some(color_map(score))))?;
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
            pr(searcher.flush(), None)?;
            break;
        };
        // keep going until we find no more matches
        match searcher.search(byte_buf[0]) {
            SearchState::Buffering => {}
            SearchState::Flushed {
                buffer: last_found_needle,
                age,
            } => {
                pr(last_found_needle, age)?;
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
            Args {
                window_size: 1024,
                age_penalty: 0.0,
            },
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
            Args {
                window_size: 1024,
                age_penalty: 0.0,
            },
        )
        .unwrap();
    }
}
