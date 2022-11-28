use crate::errors::*;
use error_chain::bail;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use std::io::BufRead;
use std::{collections::HashMap, path::PathBuf};

use crate::vfs::Folder;

#[derive(Clone, Default)]
pub struct Snippet {
    name: String,
    contents: Vec<u8>,
    metadata_path: Option<String>,
    parameters: HashMap<String, String>,
}

impl Snippet {
    pub fn process_contents(fs: &Folder, filepath: PathBuf, contents: Vec<u8>) -> Result<Vec<u8>> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\$%%\{(.*:)?(.+)\(([\w, ]*)\)\}").unwrap();
            // full example $%%{path/to/folder:snippet(val1: x, val2: y)}
            // group 1 = path/to/folder:
            // group 2 = snippet
            // group 3 = val1: x, val2: y
        }

        let mut errors = Vec::new();
        let result = RE
            .replace_all(
                std::str::from_utf8(&contents).expect("Expected file contents to be UTF8"),
                |m: &Captures| match Snippet::extract_snippet(m) {
                    Ok(mut snippet) => {
                        let mut find_filepath = filepath.iter().skip(2).collect::<PathBuf>();
                        find_filepath.set_file_name(snippet.name.clone());

                        match fs.find(find_filepath.clone()) {
                            None => panic!("snippet doesn't exist"),
                            Some((_, c)) => snippet.contents = c,
                        }

                        match snippet.process_snippet(fs) {
                            Ok(s) => s,
                            Err(e) => {
                                errors.push(e);
                                "".to_string()
                            }
                        }
                    }
                    Err(e) => {
                        errors.push(e);
                        "".to_string()
                    }
                },
            )
            .as_bytes()
            .to_vec();
        if errors.len() != 0 {
            bail!(errors
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>()
                .join("\n"));
        }
        Ok(result)
    }

    fn extract_snippet(matches: &Captures) -> Result<Snippet> {
        Ok(Snippet {
            name: match matches.get(1) {
                Some(n) => n.as_str().to_string(),
                None => bail!("snippet is missing name"),
            },
            metadata_path: match matches.get(1) {
                Some(p) => {
                    let x = p.as_str();
                    Some(x[..x.len() - 1].to_string())
                }
                None => None,
            },
            parameters: match matches.get(4) {
                Some(p) => {
                    let params = p
                        .as_str()
                        .split(",")
                        .map(|x| x.splitn(2, ":"))
                        .map(|mut x| (x.next(), x.next()))
                        .map(|x| match x {
                            (_, None) | (None, _) => Err("failed to parse parameters"),
                            (Some(px1), Some(px2)) => Ok((px1, px2)),
                        });
                    for x in params.clone() {
                        if let Err(e) = x {
                            bail!(e);
                        }
                    }
                    params
                        .map(|x| x.unwrap())
                        .map(|(x1, x2)| (x1.to_string(), x2.to_string()))
                        .collect::<HashMap<String, String>>()
                }
                None => HashMap::new(),
            },
            contents: Vec::new(),
        })
    }

    fn extract_metadata(contents: Vec<u8>) -> Result<String> {
        let mut lines = contents.lines();
        match lines.next() {
            Some(Ok(s)) => {
                if s == "---" {
                    let mut metadata = Vec::new();
                    let mut unterminated = true;
                    for l in lines {
                        let line = l.chain_err(|| "could not read file")?;
                        if line == "---" || line == "..." {
                            unterminated = false;
                            break;
                        } else {
                            metadata.push(line);
                        }
                    }
                    if !unterminated {
                        return Ok(metadata.join("\n\n"));
                    } else {
                        bail!("metadata does not terminate")
                    }
                } else {
                    return Ok(String::new());
                }
            }
            _ => return Ok(String::new()),
        }
    }

    fn process_snippet(self, fs: &Folder) -> Result<String> {
        match self.metadata_path {
            Some(ref mp) => {
                let mut snippet_result: Vec<String> = Vec::new();
                if let Err(_) = fs.map_globs(
                    &vec![mp.to_owned() + "/*"],
                    &mut |_, c| {
                        let metadata = Snippet::extract_metadata(c)?;
                        let data_map: HashMap<String, String> = serde_yaml::from_str(&metadata)
                            .chain_err(|| "Failed to parse metadata")?;
                        let mut temp = self.clone();
                        temp.parameters.extend(data_map);
                        snippet_result.push(temp.process_args()?);
                        Ok(None)
                    },
                    &mut |_, _| Ok(None),
                ) {
                    bail!("Could not find any files within the path {}", mp);
                }
                Ok(snippet_result.join("\n\n"))
            }
            None => self.process_args(),
        }
    }

    fn process_args(self) -> Result<String> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"\$%\{(.+)\}").unwrap();
        }
        let mut errors = Vec::new();
        let x = RE
            .replace_all(
                std::str::from_utf8(&self.contents)
                    .chain_err(|| "Expected snippet contents to be UTF8")?,
                |m: &Captures| match m.get(1) {
                    Some(x) => match self.parameters.get(x.as_str()) {
                        None => {
                            errors.push(format!("Argument list missing key {}", x.as_str()));
                            "".to_string()
                        }
                        Some(v) => v.clone(),
                    },
                    None => "".to_string(),
                },
            )
            .to_string();
        if errors.len() != 0 {
            bail!(errors.join("\n"));
        }
        Ok(x)
    }
}
