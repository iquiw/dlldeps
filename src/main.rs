#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;
extern crate pelite;

use pelite::FileMap;
use pelite::pe64::{Pe, PeFile};

mod errors;
use errors::*;

fn main() {
    let args = std::env::args_os();
    for arg in args.skip(1) {
        let file_map = FileMap::open(&arg).unwrap();
        let dlls = dlldeps(file_map.as_ref()).expect("invalid pe file");
        for dll in dlls {
            println!("{}", dll);
        }
    }
}

fn dlldeps(image: &[u8]) -> Result<Vec<String>> {
    let file = PeFile::from_bytes(&image)?;

    let imports = file.imports()?;
    let mut vec = vec![];
    for desc in imports {
        let dll_name = desc.dll_name()?;
        vec.push(dll_name.to_str().map(|s| s.to_string())?);
    }
    Ok(vec)
}
