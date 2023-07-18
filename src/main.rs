use std::io::{Read, Write};

use flate2::{write::DeflateEncoder, Compression};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Always);
    let mut so = stdout.lock();
    let mut e = DeflateEncoder::new(vec![], Compression::default());
    let mut text: Vec<u8> = Default::default();
    loop {
        let mut buf = vec![0u8; 1024];
        // TODO: read entire lines instead, to avoid having flushing messing things up?
        let size = si.read(&mut buf)?;
        if size == 0 {
            break;
        }
        let data = &buf[..size];
        // TODO: iterate entire utf8 characters?
        for &b in data {
            e.write_all(&[b])?;
            e.flush()?;
            text.push(b);
            let v = e.get_mut();
            let len = v.len();
            // TODO: does this mean that `data` was written, or just that the buffer before `data` was written?
            if len > 0 {
                v.clear();
                let probability = (len as f32 / text.len() as f32).clamp(0.0, 1.0);
                so.set_color(ColorSpec::new().set_fg(Some(color_map(probability))))?;
                so.write_all(&text)?;
                text.clear();
            }
        }
    }
    Ok(())
}

fn color_map(t: f32) -> Color {
    let [r, g, b, _] = colorgrad::magma().at(t.into()).to_rgba8();
    Color::Rgb(r, g, b)
}