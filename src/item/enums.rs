use colored::*;

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::{printer::TreePrintable, tree::TreeItem};

use super::structs::{Path, Visibility};

#[derive(Debug, Clone)]
pub struct Enum {
    name: String,
    vis: Visibility,
    params: Vec<String>,
    module: Path,
}

impl Display for Enum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {}::{}",
            self.vis.to_string().magenta(),
            "enum".green(),
            self.module,
            self.name.yellow(),
        )?;
        if !self.params.is_empty() {
            write!(f, "<{}>", self.params.join(","))?;
        }
        Ok(())
    }
}

impl TreeItem for Enum {
    fn module(&self) -> &Path {
        &self.module
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TreePrintable for Enum {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

impl Enum {
    fn from_syn(item: &syn::ItemEnum, module: Path) -> Self {
        let name = item.ident.to_string();
        let vis = Visibility::from_syn(&item.vis);
        let params: Vec<String> = item
            .generics
            .type_params()
            .map(|param| param.ident.to_string())
            .collect();
        Self {
            name,
            vis,
            params,
            module,
        }
    }

    pub(crate) fn renamed(&self, new_name: &str) -> Self {
        Self {
            name: String::from(new_name),
            vis: self.vis.clone(),
            params: self.params.clone(),
            module: self.module.clone(),
        }
    }

    pub(crate) fn set_visibility(&mut self, vis: Visibility) {
        self.vis = vis;
    }
}

pub fn enums_from_items(items: &[syn::Item], module: &mut Path) -> HashMap<Path, Vec<Enum>> {
    use syn::Item;
    let mut enums: HashMap<Path, Vec<Enum>> = HashMap::new();
    for item in items {
        match item {
            Item::Enum(item) => {
                let s = Enum::from_syn(item, module.clone());
                if let Some(existing_enums) = enums.get_mut(module) {
                    existing_enums.push(s);
                } else {
                    enums.insert(module.clone(), vec![s]);
                }
            }
            Item::Mod(item) => {
                module.push_name(item.ident.to_string());
                if let Some((_, content)) = &item.content {
                    let new_enums = enums_from_items(content, module);
                    enums.extend(new_enums);
                }
                module.pop();
            }
            _ => {}
        }
    }
    enums
}
