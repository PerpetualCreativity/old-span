use std::fs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use error_chain::*;
use crate::errors::*;

/// Folder is an in-memory copy of a folder.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Folder {
    pub path: PathBuf,
    pub folders: HashMap<OsString, Folder>,
    // files: HashMap<OsString, String>,
    pub files: HashMap<OsString, Vec<u8>>,
}

impl Folder {
    // Creates a new, empty, Folder, with the provided PathBuf.
    // See read() for reading a dir from a file or folder.
    pub fn new(path: PathBuf) -> Folder {
        Folder{
            path,
            folders: HashMap::new(),
            files: HashMap::new(),
        }
    }

    /// If the filename is specified in the PathBuf, reads
    /// the contents of the file into a Folder. The file
    /// must have first been written by write().
    ///
    /// If the filename is not specified, reads the contents
    /// of the specified folder into a Folder.
    pub fn read(pb: PathBuf) -> Result<Folder> {
        return if pb.is_file() {
            let contents = fs::read(pb.clone())
                .chain_err(|| format!("could not read {:?}", pb))?;
            bincode::deserialize(&contents).chain_err(|| "err")
        } else {
            let mut res = Folder::new(pb);
            let paths = fs::read_dir(res.path.clone())
                .chain_err(|| format!("could not read {:?}", res.path))?;
            for path in paths {
                let p = path.chain_err(|| "could not read file path")?;
                let file_type = p.file_type().chain_err(|| format!("could not get file type of {:?}", p))?;
                if file_type.is_dir() {
                    let mut path = res.path.clone();
                    path.push(p.file_name());
                    let folder = Self::read(path)?;
                    res.folders.insert(p.file_name(), folder);
                } else if file_type.is_file() {
                    let contents = fs::read(p.path()).chain_err(|| format!("could not read contents of {:?}", p))?;
                    res.files.insert(p.file_name(), contents);
                } else {
                    bail!(format!("span can't handle symlinks yet. Symlink found at {:?}", p.path()));
                };
            }
            Ok(res)
        }
    }
    /// Maps the provided func over the Folder's contents, returning the resulting folder.
    /// Does not modify the original folder's contents.
    pub fn map<F>(self, prefix: PathBuf, func: &mut F) -> Result<Folder> where F: FnMut(PathBuf, Vec<u8>) -> Result<(PathBuf, Vec<u8>)> {
        let mut res = Folder::new(prefix.clone());
        let mut errors = Vec::new();
        for (name, contents) in self.files.iter() {
            let mut p = self.path.clone();
            p.push(name.clone());
            let c = contents.clone();
            match func(p, c) {
                Err(e) => errors.push(e.to_string()),
                Ok(r) => {
                    res.files.insert(r.0
                                     .file_name()
                                     .ok_or("couldn't get file name")?
                                     .to_os_string()
                                     , r.1);
                },
            };
        }
        for (name, folder) in self.folders.iter() {
            let mut p = prefix.clone();
            p.push(name.clone());
            match folder.clone().map(p, func) {
                Err(e) => errors.push(e.to_string()),
                Ok(f) => {
                    res.folders.insert(name.clone(), f);
                }
            }
        }
        if !errors.is_empty() {
            bail!(errors.join("\n"));
        }
        Ok(res)
    }
    /// Gets the path to the "most matching" file.
    /// If it can't find anything, returns None.
    /// See the README for details on the algorithm.
    pub fn find(&self, file: PathBuf) -> Option<PathBuf> {
        let mut track = PathBuf::new();
        let mut folder = self;
        if let Some(parent) = file.parent() {
            for c in parent {
                match folder.folders.get(c) {
                    None => break,
                    Some(f) => {
                        folder = f;
                        track.push(c);
                    },
                }
            }
        }
        for (name, _) in folder.clone().files {
            let x = PathBuf::from(name);
            if x.file_stem() == file.file_stem() {
                return Some(track.join(x))
            }
        }
        if let Some(p) = file.parent() {
            if let Some(gp) = p.parent() {
                return self.find(gp.join(file.file_name()?))
            }
        }
        None
    }
    /// Returns a reference to the contents of the file at
    /// the specified path, returning None if the file is
    /// not found.
    pub fn get(&self, file: PathBuf) -> Option<&Vec<u8>> {
        let mut f = self;
        let file_name = file.file_name()?;
        let mut filepath = file.clone();
        filepath.pop();
        for path in &filepath {
            f = match f.folders.get(path) {
                None => return None,
                Some(folder) => folder,
            };
        }
        return match f.files.get(file_name) {
            None => None,
            Some(s) => Some(s),
        }
    }
    /// If the filename is specified in the PathBuf, the
    /// contents of the Folder are written into the specified
    /// file. This is useful for caching the result of a build.
    ///
    /// If the filename is not specified, the contents of the
    /// Folder are written into the specified folder.
    pub fn write(&self, path: PathBuf) -> Result<()> {
        if path.is_file() {
            let s = bincode::serialize(&self).chain_err(|| "couldn't serialize folder")?;
            fs::write(path.clone(), s).chain_err(|| format!("couldn't write to {:?}", path))?;
            Ok(())
        } else {
            match self.clone().map(PathBuf::new(), &mut |fp, c| {
                fs::create_dir_all(
                    fp.parent().ok_or(format!("could not get parent of path {:?}", fp))?
                ).chain_err(|| format!("could not create dirs for {:?}", fp))?;
                fs::write(fp.clone(), c.clone()).chain_err(|| format!("couldn't write to {:?}", fp))?;
                Ok((fp, c))
            }) {
                Err(e) => Err(e.chain_err(|| format!("error(s) when writing folder to {:?}", path))),
                Ok(_) => Ok(()),
            }
        }
    }
}
