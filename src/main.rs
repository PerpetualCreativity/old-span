use clap::Parser;
use std::{fs, env, path};

mod vfs;
mod args;
mod build;
mod errors { error_chain::error_chain!{} }

use errors::*;

fn main() {
    if let Err(ref e) = run() {
        eprintln!("error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        std::process::exit(1);
    }
}

fn run() -> Result <()> {
    let args = args::Args::parse();
    match args.command {
        args::Command::Build{ input, output } => {
            let cwd = env::current_dir().chain_err(|| "could not access current directory")?;
            env::set_current_dir(input.clone()).chain_err(|| format!("could not set directory to {:?}", input))?;
            let source = vfs::Folder::read(path::PathBuf::from("."))?;
            let result = build::build(source)?;
            env::set_current_dir(cwd).chain_err(|| format!("could not set directory to {:?}", input))?;
            if fs::metadata(output.clone()).is_err() {
                fs::remove_dir_all(output.clone()).chain_err(|| format!(
                    "could not delete previous output at {:?}",
                    output.clone()
                ))?;
                fs::create_dir(output.clone()).chain_err(|| format!(
                    "could not create output directory {:?}",
                    output.clone()
                ))?;
            }
            env::set_current_dir(output.clone()).chain_err(|| format!("could not set directory to {:?}", output))?;
            result.write(output)
        },
        args::Command::Serve{ input: _, port: _ } => error_chain::bail!("serve is not implemented yet"),
    }
}
