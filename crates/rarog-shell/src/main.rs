use rarog_engine::{RenderOptions, render_html};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rarog-shell: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input = args.next().unwrap_or_else(|| "examples/hello.html".into());
    let output = args.next().unwrap_or_else(|| "rarog.ppm".into());
    let source = fs::read_to_string(&input)?;
    let rendered = render_html(&source, RenderOptions::default())?;
    fs::write(&output, rendered.framebuffer.to_ppm())?;
    println!("Rarog rendered {input} -> {output}");
    println!("display commands: {}", rendered.display_list.commands.len());
    Ok(())
}
