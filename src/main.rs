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
    /// How long back in history to look for matches.
    ///
    /// The further back in history a match is found, the lower its compressibility score is.
    #[arg(long, default_value_t = 1024)]
    window_size: usize,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    assert!(args.window_size > 0);
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
    let Args { window_size } = args;
    // refcell so we can reset on scope exit
    let soc = RefCell::new(so);
    defer! {
        soc.borrow_mut().reset().unwrap();
    }
    let pr = |buffered: VecDeque<u8>, score: f32| -> anyhow::Result<()> {
        // score of 1 means buffered is uncompressible, 0 means it is fully compressible
        debug_assert!((0.0..=1.0).contains(&score));
        if !buffered.is_empty() {
            //let score = 1f32 / (buffered.len() as f32 + age.unwrap_or(0) as f32 * age_penalty);
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
            pr(searcher.flush(), 0.0)?;
            break;
        };
        // keep going until we find no more matches
        match searcher.search(byte_buf[0]) {
            SearchState::Buffering => {}
            SearchState::Flushed { buffer, age } => {
                let score = 1.0
                    / 1f32.max(
                        buffer.len() as f32
                            / (1.0 - (age.unwrap_or(0) as f32 / window_size as f32)),
                    );
                pr(buffer, score)?;
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
            Args { window_size: 1024 },
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
            Args { window_size: 1024 },
        )
        .unwrap();
    }
}
