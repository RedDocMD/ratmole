use crate::printer::TreePrintable;
use crate::tree::TreeItem;

use super::structs::{Path, Visibility};
use colored::*;

use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct Const {
    name: String,
    vis: Visibility,
    module: Path,
}

impl Display for Const {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {}",
            self.vis.to_string().magenta(),
            "const".green(),
            self.name
        )
    }
}

impl TreeItem for Const {
    fn module(&self) -> &Path {
        &self.module
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TreePrintable for Const {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

impl Const {
    fn from_syn(item: &syn::ItemConst, module: Path) -> Self {
        let name = item.ident.to_string();
        let vis = Visibility::from_syn(&item.vis);
        Self { name, vis, module }
    }
}

pub fn consts_from_items(items: &[syn::Item], module: &mut Path) -> HashMap<Path, Vec<Const>> {
    use syn::Item;
    let mut consts: HashMap<Path, Vec<Const>> = HashMap::new();
    for item in items {
        match item {
            Item::Const(item) => {
                let s = Const::from_syn(item, module.clone());
                if let Some(existing_consts) = consts.get_mut(module) {
                    existing_consts.push(s);
                } else {
                    consts.insert(module.clone(), vec![s]);
                }
            }
            Item::Mod(item) => {
                module.push_name(item.ident.to_string());
                if let Some((_, content)) = &item.content {
                    let new_consts = consts_from_items(content, module);
                    consts.extend(new_consts);
                }
                module.pop();
            }
            _ => {}
        }
    }
    consts
}
