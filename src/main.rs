#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate pelite;

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use pelite::FileMap;
use pelite::pe64::{Pe, PeFile};

mod errors;
use errors::*;

enum DllDepResult {
    NotFound,
    Found(Vec<OsString>),
}

fn main() {
    let args = std::env::args_os();
    let mut dep_map: HashMap<OsString, DllDepResult> = HashMap::new();
    let mut remain_files: Vec<PathBuf> = args
        .skip(1)
        .filter_map(|file| {
            if let Ok(pathbuf) = Path::new(&file).canonicalize() {
                Some(pathbuf)
            } else {
                println!("File Not Found: {}", file.to_string_lossy());
                None
            }
        })
        .collect();

    while let Some(dll_pathbuf) = remain_files.pop() {
        let dlls = find_deps(&dll_pathbuf).expect("invalid pe file");
        for dll in &dlls {
            if let Ok(dep_pathbuf) = find_dll(dll) {
                if !dep_map.contains_key(dll) {
                    remain_files.push(dep_pathbuf);
                }
            } else {
                dep_map.insert(dll.to_os_string(), DllDepResult::NotFound);
            }
        }
        dep_map.insert(OsString::from(dll_pathbuf.file_name().unwrap()),
                       DllDepResult::Found(dlls));
    }
    for (k, v) in &dep_map {
        match v {
            &DllDepResult::Found(ref v) => {
                println!("{}", k.to_string_lossy());
                for d in v {
                    println!(" -> {}", d.to_string_lossy());
                }
            },
            &DllDepResult::NotFound => println!("{} -> NOTFOUND", k.to_string_lossy()),
        }
    }
}

fn find_dll<S: AsRef<OsStr>>(name: &S) -> Result<PathBuf> {
    Ok(Path::new(name).canonicalize()?)
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
