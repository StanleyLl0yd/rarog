use rarog_engine::{RenderOptions, render_html};
use rarog_types::Size;
use std::{
    env, fs, io,
    path::PathBuf,
    process::{ExitCode, Termination},
};

struct Arguments {
    input: PathBuf,
    output: PathBuf,
    width: u32,
    height: u32,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rarog-r6-wpt-render: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = parse_arguments(env::args().skip(1))?;
    let source = fs::read_to_string(&arguments.input)?;
    let rendered = render_html(
        &source,
        RenderOptions {
            viewport: Size {
                width: arguments.width as f32,
                height: arguments.height as f32,
            },
            ..RenderOptions::default()
        },
    )?;
    fs::write(arguments.output, rendered.framebuffer.to_ppm())?;
    Ok(())
}

fn parse_arguments(
    arguments: impl Iterator<Item = String>,
) -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut input = None;
    let mut output = None;
    let mut width = None;
    let mut height = None;
    let mut arguments = arguments;

    while let Some(flag) = arguments.next() {
        let value = arguments.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{flag} requires a value"),
            )
        })?;
        match flag.as_str() {
            "--input" if input.is_none() => input = Some(PathBuf::from(value)),
            "--output" if output.is_none() => output = Some(PathBuf::from(value)),
            "--width" if width.is_none() => width = Some(parse_dimension("width", &value)?),
            "--height" if height.is_none() => height = Some(parse_dimension("height", &value)?),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown or duplicate argument {flag}"),
                )
                .into());
            }
        }
    }

    Ok(Arguments {
        input: input.ok_or_else(|| missing("--input"))?,
        output: output.ok_or_else(|| missing("--output"))?,
        width: width.ok_or_else(|| missing("--width"))?,
        height: height.ok_or_else(|| missing("--height"))?,
    })
}

fn missing(flag: &'static str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("missing required argument {flag}"),
    )
}

fn parse_dimension(label: &'static str, value: &str) -> Result<u32, io::Error> {
    let parsed = value.parse::<u32>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} must be a positive integer"),
        )
    })?;
    if parsed == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} must be greater than zero"),
        ));
    }
    Ok(parsed)
}
