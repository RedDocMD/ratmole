use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::{printer::TreePrintable, tree::TreeItem};

use super::structs::Path;

pub struct Module {
    path: Path,
    name: String,
    parent: Path,
}

impl Module {
    pub fn new(names: &[String]) -> Self {
        let path = Path::from(names.to_vec());
        let parent = path.parent();
        Self {
            path,
            name: names.last().unwrap().clone(),
            parent,
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl TreePrintable for Module {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.components().last().unwrap())
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

impl TreeItem for Module {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &Path {
        &self.parent
    }
}

pub fn modules_from_items(items: &[syn::Item], module: &mut Path) -> HashMap<Path, Vec<Module>> {
    use syn::Item;
    let mut modules: HashMap<Path, Vec<Module>> = HashMap::new();
    for item in items {
        match item {
            Item::Mod(item) => {
                let parent = module.clone();
                module.push_name(item.ident.to_string());
                let new_module = Module {
                    path: module.clone(),
                    parent: parent.clone(),
                    name: item.ident.to_string(),
                };
                if let Some(existing_modules) = modules.get_mut(&parent) {
                    existing_modules.push(new_module);
                } else {
                    modules.insert(parent, vec![new_module]);
                }
                if let Some((_, content)) = &item.content {
                    let mut new_structs = modules_from_items(content, module);
                    modules.extend(new_structs);
                }
                module.pop();
            }
            _ => {}
        }
    }
    modules
}
