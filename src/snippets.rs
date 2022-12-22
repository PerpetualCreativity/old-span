use crate::errors::*;
use error_chain::bail;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
use serde_yaml::{Mapping, Value};
use std::io::BufRead;
use std::path::PathBuf;

use crate::vfs::Folder;

/// Contains snippet-related data.
#[derive(Clone, Default)]
pub struct Snippet {
    name: String,
    contents: Vec<u8>,
    metadata_path: Option<String>,
    parameters: Mapping,
}

impl Snippet {
    /// Processes the contents of a source file that might include snippets.
    /// Returns the source file with snippet syntax replaced by the expanded snippet.
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

    /// Extracts snippet data from a regex match.
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
                    let mut params = Mapping::new();
                    for x in p.as_str().split(",") {
                        let mut name_args = x.splitn(2, ":");
                        let name = name_args.next().ok_or("failed to parse parameters")?;
                        let arg = name_args.next().ok_or("failed to parse parameters")?;
                        params.insert(
                            Value::String(name.to_string()),
                            Value::String(arg.to_string()),
                        );
                    }
                    params
                }
                None => Mapping::new(),
            },
            contents: Vec::new(),
        })
    }

    /// Gets and parses YAML metadata from the source file.
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

    /// Processes a snippet using the source folder.
    /// Returns snippet expansion (including metadata expansion if necessary).
    fn process_snippet(self, fs: &Folder) -> Result<String> {
        match self.metadata_path {
            Some(ref mp) => {
                let mut snippet_result: Vec<String> = Vec::new();
                if let Err(_) = fs.map_globs(
                    &vec![mp.to_owned() + "/*"],
                    &mut |_, c| {
                        let metadata = Snippet::extract_metadata(c)?;
                        let data_map: Mapping = serde_yaml::from_str(&metadata)
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
                    Some(mat) => {
                        let mut current: Value = Value::Mapping(self.parameters.clone());
                        for v in mat.as_str().split(".") {
                            let gr;
                            let k = v.parse::<usize>();
                            match k {
                                Ok(x) => gr = current.get(x),
                                Err(_) => gr = current.get(v),
                            }

                            match gr {
                                None | Some(Value::Null) => {
                                    errors.push(format!(
                                        "Key {} does not exist (part of key chain \"{}\")",
                                        v,
                                        mat.as_str()
                                    ));
                                    return "".to_string();
                                }
                                Some(Value::Tagged(x)) => current = x.clone().value,
                                Some(x) => current = x.clone(),
                            }
                        }

                        while let Value::Tagged(x) = current {
                            // unwrap tagged value
                            current = x.value;
                        }

                        match current {
                            Value::Null => "".to_string(),
                            Value::Bool(x) => x.to_string(),
                            Value::Number(x) => x.to_string(),
                            Value::String(x) => x,
                            Value::Sequence(x) => {
                                errors.push(format!(
                                    "Value is a sequence and therefore cannot be treated like a string: {:#?}",
                                    x,
                                ));
                                "".to_string()
                            },
                            Value::Mapping(x) => {
                                errors.push(format!(
                                    "Value is a mapping and therefore cannot be treated like a string: {:#?}",
                                    x,
                                ));
                                "".to_string()
                            },
                            Value::Tagged(x) => {
                                errors.push(format!(
                                    "Value is a mapping and therefore cannot be treated like a string: {:#?}",
                                    x,
                                ));
                                "".to_string()
                            },
                        }
                    }
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
