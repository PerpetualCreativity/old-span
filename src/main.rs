use clap::Parser;
use std::{env, fs, path};

mod args;
mod build;
mod config;
mod snippets;
mod vfs;
mod errors {
    error_chain::error_chain! {}
}

use errors::*;

fn main() {
    if let Err(ref e) = run() {
        eprintln!("error: {}", e);
        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = args::Args::parse();
    match args.command {
        args::Command::Build { input, output } => {
            let cwd = env::current_dir().chain_err(|| "could not access current directory")?;
            env::set_current_dir(input.clone())
                .chain_err(|| format!("could not set directory to {:?}", input))?;
            let source = vfs::Folder::read(path::PathBuf::from("."))?;
            let config_file =
                fs::File::open("./span.yml").chain_err(|| "could not open ./span.yml")?;
            let config = serde_yaml::from_reader(config_file)
                .chain_err(|| "./span.yml contains invalid config syntax")?;
            let result = build::build(source, config)?;
            env::set_current_dir(cwd)
                .chain_err(|| format!("could not set directory to {:?}", input))?;
            if fs::metadata(output.clone()).is_ok() {
                fs::remove_dir_all(output.clone()).chain_err(|| {
                    format!("could not delete previous output at {:?}", output.clone())
                })?;
            }
            fs::create_dir(output.clone())
                .chain_err(|| format!("could not create output directory {:?}", output.clone()))?;
            env::set_current_dir(output.clone())
                .chain_err(|| format!("could not set directory to {:?}", output))?;
            result.write(output)
        }
        args::Command::Serve { input: _, port: _ } => {
            error_chain::bail!("serve is not implemented yet")
        }
    }
}
