use crate::config;
use crate::errors::*;
use crate::snippets;
use crate::vfs::Folder;
use error_chain::bail;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

fn run_command(command: String, stdin: Vec<u8>, err_context: String) -> Result<Output> {
    let mut split = command.split_ascii_whitespace().map(str::to_owned);
    let program = split.next().ok_or(format!(
        "invalid command string {}: could not find program name{}",
        command, err_context
    ))?;
    let mut c = Command::new(program.clone());

    let mut stdin_file = None;
    let mut stdout_file = None;
    let tmp_dir = TempDir::new().chain_err(|| "couldn't create temporary directory")?;

    let args = split
        .map(|x| match &*x {
            "%i" => {
                let f = tmp_dir.path().join("stdin");
                stdin_file = Some(f.clone());
                f.into()
            }
            "%o" => {
                let f = tmp_dir.path().join("stdout");
                stdout_file = Some(f.clone());
                f.into()
            }
            _ => OsString::from(x),
        })
        .collect::<Vec<OsString>>();
    c.args(args);

    if let Some(path) = stdin_file {
        fs::write(path, stdin.clone())
            .chain_err(|| format!("failed to write to temporary input file"))?;
    }

    let mut child = c
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .chain_err(|| format!("failed to start {}{}", program.clone(), err_context))?;
    let mut stdin_handle = child
        .stdin
        .take()
        .chain_err(|| format!("failed to get {} stdin{}", program.clone(), err_context))?;
    let ec = err_context.clone();
    let p = program.clone();
    std::thread::spawn(move || {
        stdin_handle
            .write_all(&stdin)
            .chain_err(|| format!("failed to write to {} stdin{}", p, ec))
    });
    let mut output = child
        .wait_with_output()
        .chain_err(|| format!("failed to wait on {}{}", program, err_context));

    if let Ok(ref mut w) = output {
        if let Some(path) = stdout_file {
            w.stdout = fs::read(path)
                .chain_err(|| format!("failed to read from temporary output file"))?;
        }
    }
    output
}

pub fn pandoc(
    folder: Folder,
    filters: Vec<config::Filter>,
    extra_args: Vec<String>,
    default: String,
) -> Result<Folder> {
    // TODO: process snippets
    let mut command = String::from("pandoc --to html5 --standalone ");
    command.push_str(&extra_args.join(" "));
    let contents_fs = folder
        .folders
        .get(&OsString::from("contents"))
        .chain_err(|| "Could not find folder 'contents'")?;
    let templates_fs = folder
        .folders
        .get(&OsString::from("templates"))
        .chain_err(|| "Could not find folder 'templates'")?;
    let snippets_fs = folder
        .folders
        .get(&OsString::from("snippets"))
        .chain_err(|| "Could not find folder 'snippets'")?;
    contents_fs
        .clone()
        .map(PathBuf::new(), &mut |filepath, contents| {
            let mut template = templates_fs.clone().path;
            let mut find_filepath: PathBuf = filepath.iter().skip(2).collect();
            template.push(match templates_fs.find(find_filepath.clone()) {
                None => {
                    find_filepath.set_file_name(default.clone());
                    match templates_fs.find(find_filepath.clone()) {
                        None => bail!("failed to find a matching template for {:?}", find_filepath),
                        Some((fp, _)) => fp,
                    }
                }
                Some((fp, _)) => fp,
            });
            let err_context = format!(
                ", while processing file {:?}, using template {:?}",
                filepath,
                template.clone(),
            );
            let mut child = command.clone();
            child.push_str(" --template ");
            child.push_str(&template.to_string_lossy());

            for filter in filters.iter() {
                if contents_fs
                    .get_globs(&filter.files)?
                    .keys()
                    .any(|e| *e == filepath)
                {
                    child.push_str("--filter=");
                    child.push_str(
                        filter
                            .path
                            .to_str()
                            .ok_or(format!("couldn't get path of filter"))?,
                    );
                }
            }

            let output = run_command(
                child,
                snippets::Snippet::process_contents(snippets_fs, filepath.clone(), contents)?,
                err_context.clone(),
            )?;

            if !output.stderr.is_empty() {
                bail!(
                    "error from pandoc{}:\n{}",
                    err_context,
                    String::from_utf8_lossy(&output.stderr),
                )
            }

            Ok(Some((
                filepath
                    .join(
                        filepath
                            .file_stem()
                            .ok_or(format!("couldn't get file stem of {:?}", filepath))?,
                    )
                    .join(PathBuf::from(".html")),
                output.stdout,
            )))
        })
        .chain_err(|| "failed to build")
}

pub fn build(folder: Folder, config: config::Config) -> Result<Folder> {
    let mut f = folder;
    f = f.remove_globs(&config.ignore)?;
    for pr in config.pre_run {
        f = f.map_globs(
            &pr.files,
            &mut |fp, c| {
                let err_context = format!(", while processing file {:?}", fp);
                let output = run_command(pr.command.clone(), c.clone(), err_context.clone())?;
                if pr.error_on != "none" {
                    if pr.error_on == "stderr" && !output.stderr.is_empty() {
                        bail!(
                            "error from pre-run command ({}) {}:\n  {}",
                            pr.command,
                            err_context,
                            String::from_utf8_lossy(&output.stderr),
                        )
                    } else if pr.error_on == "stdout" && !output.stdout.is_empty() {
                        bail!(
                            "error from pre-run command ({}) {}:\n  {}",
                            pr.command,
                            err_context,
                            String::from_utf8_lossy(&output.stdout),
                        )
                    } else if pr.error_on == "status" && output.status.success() {
                        bail!(
                            "error from pre-run command ({}) {}:\n  {}",
                            pr.command,
                            err_context,
                            String::from_utf8_lossy(&output.stdout),
                        )
                    }
                }
                if pr.replace {
                    Ok(Some((fp, output.stdout)))
                } else {
                    Ok(Some((fp, c)))
                }
            },
            &mut |fp, c| Ok(Some((fp, c))),
        )?;
    }
    let pass = f.filter_globs(&config.passthrough)?;
    f = f.remove_globs(&config.passthrough)?;
    f = pandoc(
        f,
        config.filters,
        config.extra_args,
        config.default_template,
    )?;
    f = Folder::join(pass, f)?;
    Ok(f)
}
