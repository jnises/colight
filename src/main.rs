use std::io::{Write, Read};

use termcolor::{StandardStream, WriteColor, ColorChoice, Color, ColorSpec};

fn main() -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut si = stdin.lock();
    let stdout = StandardStream::stdout(ColorChoice::Always);
    //let stdout = std::io::stdout();
    let mut so = stdout.lock();
    loop {
        let mut buf = vec![0u8; 1024];
        let size = si.read(&mut buf)?;
        let data = &buf[..size];
        so.set_color(ColorSpec::new().set_fg(Some(color_map(0.5))))?;
        so.write_all(&data)?;
    }
}

fn color_map(t: f32) -> Color {
    let [r, g, b, _] = colorgrad::spectral().at(t.into()).to_rgba8();
    Color::Rgb(r, g, b)
}

