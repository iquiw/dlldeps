#![recursion_limit = "1024"]

extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate pelite;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, Command};
use pelite::pe64::{Pe, PeFile};
use pelite::FileMap;

mod errors;
use errors::*;

enum DllDepResult {
    NotFound,
    Found(Vec<OsString>),
    Invalid(Error),
    Queued,
}

fn main() {
    let matches = Command::new("DllDeps")
        .about("DLL dependency resolver")
        .arg(
            Arg::new("dirs")
                .short('d')
                .value_name("DIR")
                .action(ArgAction::Append)
                .num_args(1)
                .help("Specify directory where DLL is searched"),
        )
        .arg(
            Arg::new("found-only")
                .short('f')
                .long("found-only")
                .action(ArgAction::SetTrue)
                .help("Show found only"),
        )
        .arg(
            Arg::new("long")
                .short('l')
                .long("long")
                .action(ArgAction::SetTrue)
                .help("Show DLL dependency arrow"),
        )
        .arg(
            Arg::new("dlls")
                .value_name("DLL")
                .action(ArgAction::Append)
                .num_args(1)
                .required(true),
        )
        .get_matches();
    let found_only = matches.contains_id("found-only");
    let show_long = matches.contains_id("long");
    let search_dirs = matches
        .get_many::<String>("dirs")
        .unwrap_or_default()
        .collect::<Vec<_>>();
    let mut remain_files: Vec<PathBuf> = matches
        .get_many::<String>("dlls")
        .unwrap_or_default()
        .filter_map(|file| {
            if let Ok(pathbuf) = Path::new(&file).canonicalize() {
                Some(pathbuf)
            } else {
                println!("File Not Found: {}", file);
                None
            }
        })
        .collect();

    let mut dep_map: HashMap<PathBuf, DllDepResult> = HashMap::new();
    while let Some(dll_pathbuf) = remain_files.pop() {
        match find_deps(&dll_pathbuf) {
            Ok(dlls) => {
                for dll in &dlls {
                    if let Some(dep_pathbuf) = find_dll(&search_dirs, dll) {
                        dep_map.entry(dep_pathbuf.clone()).or_insert_with(|| {
                            remain_files.push(dep_pathbuf);
                            DllDepResult::Queued
                        });
                    } else {
                        dep_map.insert(PathBuf::from(dll), DllDepResult::NotFound);
                    }
                }
                dep_map.insert(dll_pathbuf, DllDepResult::Found(dlls));
            }
            Err(err) => {
                dep_map.insert(dll_pathbuf, DllDepResult::Invalid(err));
            }
        }
    }
    for (k, v) in &dep_map {
        match v {
            DllDepResult::Found(ref v) => {
                println!("{}", k.to_string_lossy());
                if show_long {
                    for d in v {
                        println!(" -> {}", d.to_string_lossy());
                    }
                }
            }
            DllDepResult::NotFound => {
                if !found_only {
                    println!("{} (NOTFOUND)", k.to_string_lossy());
                }
            }
            DllDepResult::Invalid(err) => println!("{} (Error: {})", k.to_string_lossy(), err),
            _ => {}
        }
    }
}

fn find_dll<S, T>(dirs: &Vec<S>, name: &T) -> Option<PathBuf>
where
    S: AsRef<OsStr>,
    T: AsRef<Path>,
{
    for dir in dirs {
        let mut pathbuf = PathBuf::from(dir);
        pathbuf.push(name);
        if let Ok(p) = pathbuf.canonicalize() {
            return Some(p);
        }
    }
    None
}

fn find_deps(path: &Path) -> Result<Vec<OsString>> {
    let file_map = FileMap::open(path).unwrap();
    let file = PeFile::from_bytes(&file_map)?;
    let mut vec = vec![];
    for desc in file.imports()? {
        let dll_name = desc.dll_name()?;
        vec.push(dll_name.to_str().map(OsString::from)?);
    }
    Ok(vec)
}
