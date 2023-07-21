use std::io::{BufRead, Write};

use flate2::{write::DeflateEncoder, Compression};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Always);
    let mut so = stdout.lock();
    let mut e = DeflateEncoder::new(vec![], Compression::default());
    let mut line = String::new();
    loop {
        line.clear();
        if si.read_line(&mut line)? == 0 {
            break;
        }
        e.write_all(line.as_bytes())?;
        e.flush()?;
        let compression = e.total_out() as f32 / e.total_in() as f32;
        // TODO: test without this line
        e.get_mut().clear();
        so.set_color(ColorSpec::new().set_fg(Some(color_map(compression))))?;
        so.write_all(line.as_bytes())?;
    }
    Ok(())
}

fn color_map(t: f32) -> Color {
    let [r, g, b, _] = colorgrad::magma().at(t.into()).to_rgba8();
    Color::Rgb(r, g, b)
}
