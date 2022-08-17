use std::ffi::OsString;
use std::path::PathBuf;
use std::io::Write;
use std::process::{Command, Stdio};
use error_chain::bail;
use crate::vfs::Folder;
use crate::errors::*;

pub fn build(folder: Folder) -> Result<Folder> {
    let mut command = Command::new("pandoc");
    command
        .args(&["--to", "html5", "--standalone"]);
    let contents_fs = folder.folders.get(&OsString::from("contents")).chain_err(|| "Could not find folder 'contents'")?;
    let templates_fs = folder.folders.get(&OsString::from("templates")).chain_err(|| "Could not find folder 'templates'")?;
    contents_fs.clone().map(PathBuf::new(),&mut |filepath, contents| {
        let mut template = folder.clone().path;
        template.push("templates");
        let mut find_filepath: PathBuf = filepath.iter().skip(2).collect();
        template.push(match templates_fs.find(find_filepath.clone()) {
            None => {
                find_filepath.set_file_name("default.html");
                match templates_fs.find(find_filepath.clone()) {
                    None => bail!(format!("failed to find a matching template for {:?}", find_filepath)),
                    Some(fp) => fp,
                }
            },
            Some(fp) => fp,
        });
        let err_context = format!(
            ", while processing file {:?}, using template {:?}",
            filepath,
            template.clone(),
        );
        let mut child = command
            .arg("--template").arg(OsString::from(template))
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn()
            .chain_err(|| format!("failed to start pandoc{}", err_context))?;
        let mut stdin = child.stdin.take().chain_err(|| format!("failed to get pandoc stdin{}", err_context))?;
        let c = contents;
        let ec = err_context.clone();
        std::thread::spawn(move || {
            stdin.write_all(&c).chain_err(|| format!("failed to write to pandoc stdin{}", ec))
        });
        let output = child
            .wait_with_output()
            .chain_err(|| format!("failed to wait on pandoc{}", err_context))?;
        if !output.stderr.is_empty() {
            bail!(format!(
                "error from pandoc{}:\n  {}",
                err_context,
                String::from_utf8_lossy(&output.stderr),
            ))
        }

        Ok((
            filepath
                .join(filepath.file_stem().ok_or(format!("couldn't get file stem of {:?}", filepath))?)
                .join(PathBuf::from(".html")),
            output.stdout))
    }).chain_err(|| "failed to build")
}
