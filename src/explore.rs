use regex::Regex;
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

use crate::{
    error::Error,
    structs::{structs_from_items, Path, Struct},
};

pub fn structs_in_crate<T: AsRef<std::path::Path>>(crate_path: T) -> Result<Vec<Struct>, Error> {
    let mut src_path = PathBuf::from(crate_path.as_ref());
    src_path.push("src");
    let mut structs = Vec::new();
    for entry in fs::read_dir(src_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
            if is_rust_filename(name) {
                let mod_name = &name[..name.len() - 3];
                structs.append(&mut structs_from_file(
                    &path,
                    Path::from(vec![String::from(mod_name)]),
                )?);
            }
        }
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let name = path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
                    if name == "mod.rs" {
                        structs.append(&mut structs_from_file(
                            path,
                            Path::from(vec![String::from(dir_name)]),
                        )?);
                    } else if is_rust_filename(name) {
                        let mod_name = &name[..name.len() - 3];
                        structs.append(&mut structs_from_file(
                            &path,
                            Path::from(vec![String::from(dir_name), String::from(mod_name)]),
                        )?);
                    }
                }
            }
        }
    }
    Ok(structs)
}

fn is_rust_filename(name: &str) -> bool {
    lazy_static! {
        static ref RS_REG: Regex = Regex::new(r"^[^.]+\.rs$").unwrap();
    }
    RS_REG.is_match(name)
}

fn structs_from_file<T: AsRef<std::path::Path>>(
    file_path: T,
    module: crate::structs::Path,
) -> Result<Vec<Struct>, Error> {
    let mut file = File::open(file_path.as_ref())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let ast = syn::parse_file(&contents)?;
    Ok(structs_from_items(&ast.items, module))
}
