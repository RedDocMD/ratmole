use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use super::structs::{Path, Visibility};

pub struct ExternCrate {
    name: String,
    rename: Option<String>,
    module: Path,
    vis: Visibility,
}

impl Display for ExternCrate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} extern crate {}", self.vis, self.name)?;
        if let Some(rename) = &self.rename {
            write!(f, " as {}", rename)?;
        }
        Ok(())
    }
}

impl ExternCrate {
    pub fn rename(&self) -> &Option<String> {
        &self.rename
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

pub fn extern_crates_from_items(
    items: &[syn::Item],
    module: &mut Path,
) -> HashMap<Path, Vec<ExternCrate>> {
    let mut extern_crates_map: HashMap<Path, Vec<ExternCrate>> = HashMap::new();
    for item in items {
        match item {
            syn::Item::ExternCrate(item) => {
                let new_crate = ExternCrate {
                    name: item.ident.to_string(),
                    vis: Visibility::from_syn(&item.vis),
                    module: module.clone(),
                    rename: item.rename.as_ref().map(|(_, name)| name.to_string()),
                };
                if let Some(existing_crates) = extern_crates_map.get_mut(module) {
                    existing_crates.push(new_crate);
                } else {
                    extern_crates_map.insert(module.clone(), vec![new_crate]);
                }
            }
            syn::Item::Mod(item) => {
                if let Some((_, items)) = item.content.as_ref() {
                    module.push_name(item.ident.to_string());
                    let new_crates = extern_crates_from_items(items, module);
                    for (k, v) in new_crates {
                        extern_crates_map.insert(k, v);
                    }
                    module.pop();
                }
            }
            _ => {}
        }
    }
    extern_crates_map
}
