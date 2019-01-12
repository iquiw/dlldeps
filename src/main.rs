#![recursion_limit = "1024"]

extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate pelite;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use clap::{App, Arg};
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
    let matches = App::new("DllDeps")
        .about("DLL dependency resolver")
        .arg(
            Arg::with_name("dirs")
                .short("d")
                .value_name("DIR")
                .help("Specify directory where DLL is searched")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("found-only")
                .short("f")
                .long("found-only")
                .help("Show found only"),
        )
        .arg(
            Arg::with_name("long")
                .short("l")
                .long("long")
                .help("Show DLL dependency arrow"),
        )
        .arg(
            Arg::with_name("dlls")
                .value_name("DLL")
                .multiple(true)
                .required(true),
        )
        .get_matches();
    let found_only = matches.is_present("found-only");
    let show_long = matches.is_present("long");
    let search_dirs = matches.values_of_os("dirs").unwrap_or_default().collect();
    let mut remain_files: Vec<PathBuf> = matches
        .values_of_os("dlls")
        .unwrap_or_default()
        .filter_map(|file| {
            if let Ok(pathbuf) = Path::new(&file).canonicalize() {
                Some(pathbuf)
            } else {
                println!("File Not Found: {}", file.to_string_lossy());
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
                        if !dep_map.contains_key(&dep_pathbuf) {
                            remain_files.push(dep_pathbuf.clone());
                            dep_map.insert(dep_pathbuf, DllDepResult::Queued);
                        }
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
            &DllDepResult::Found(ref v) => {
                println!("{}", k.to_string_lossy());
                if show_long {
                    for d in v {
                        println!(" -> {}", d.to_string_lossy());
                    }
                }
            }
            &DllDepResult::NotFound => {
                if !found_only {
                    println!("{} (NOTFOUND)", k.to_string_lossy());
                }
            }
            &DllDepResult::Invalid(ref err) => println!("{} (Error: {})", k.to_string_lossy(), err),
            _ => {}
        }
    }
}

fn find_dll<'a, S>(dirs: &Vec<&'a OsStr>, name: &S) -> Option<PathBuf>
where
    S: AsRef<Path>,
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
        vec.push(dll_name.to_str().map(|s| OsString::from(s))?);
    }
    Ok(vec)
}
