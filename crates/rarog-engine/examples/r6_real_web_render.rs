use rarog_dom::{Document, Namespace, NodeId, NodeKind};
use rarog_engine::{RenderOptions, render_html};
use rarog_types::Size;
use std::{env, fs, io, path::PathBuf, process::ExitCode};

struct Arguments {
    input: PathBuf,
    screenshot: PathBuf,
    title: PathBuf,
    width: u32,
    height: u32,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rarog-r6-real-web-render: {error}");
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
    fs::write(arguments.screenshot, rendered.framebuffer.to_ppm())?;
    fs::write(
        arguments.title,
        document_title(&rendered.document).unwrap_or_default(),
    )?;
    Ok(())
}

fn document_title(document: &Document) -> Option<String> {
    let mut stack = vec![document.root()];
    while let Some(id) = stack.pop() {
        let node = document.node(id)?;
        if let NodeKind::Element(element) = &node.kind {
            if element.namespace == Namespace::Html && element.tag_name.as_str() == "title" {
                let mut text = String::new();
                collect_text(document, id, &mut text);
                let normalized = text.split_ascii_whitespace().collect::<Vec<_>>().join(" ");
                return Some(normalized);
            }
        }
        stack.extend(node.children.iter().rev().copied());
    }
    None
}

fn collect_text(document: &Document, id: NodeId, output: &mut String) {
    let Some(node) = document.node(id) else {
        return;
    };
    if let NodeKind::Text(text) = &node.kind {
        output.push_str(text);
        return;
    }
    for child in &node.children {
        collect_text(document, *child, output);
    }
}

fn parse_arguments(
    arguments: impl Iterator<Item = String>,
) -> Result<Arguments, Box<dyn std::error::Error>> {
    let mut input = None;
    let mut screenshot = None;
    let mut title = None;
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
            "--screenshot" if screenshot.is_none() => screenshot = Some(PathBuf::from(value)),
            "--title" if title.is_none() => title = Some(PathBuf::from(value)),
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
        screenshot: screenshot.ok_or_else(|| missing("--screenshot"))?,
        title: title.ok_or_else(|| missing("--title"))?,
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
