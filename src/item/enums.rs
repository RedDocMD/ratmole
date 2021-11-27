use colored::*;

use std::fmt::{self, Display, Formatter};

use crate::{from_items, printer::TreePrintable, tree::TreeItem};

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

from_items!(enums_from_items, Enum, Enum);
