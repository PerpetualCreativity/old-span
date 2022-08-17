use std::path::PathBuf;

/// A static site generator based on pandoc.
#[derive(clap::Parser, Debug)]
#[clap(author, version, about, long_about=None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(short, long, value_parser, default_value="span.yml", global=true)]
    pub config: PathBuf,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Build {
        /// Path to source files.
        #[clap(default_value=".", forbid_empty_values=true)]
        input: PathBuf,

        /// Path to output folder.
        #[clap(default_value="./output", forbid_empty_values=true)]
        output: PathBuf,
    },
    Serve {
        /// Path to source files.
        #[clap(default_value=".", forbid_empty_values=true)]
        input: PathBuf,

        /// Port to run the web server on.
        #[clap(short, long, value_parser, default_value_t=3000)]
        port: u16,
    }
}

