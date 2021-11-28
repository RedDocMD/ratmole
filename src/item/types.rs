use colored::*;

use std::fmt::{self, Display, Formatter};

use crate::{from_items, printer::TreePrintable, tree::TreeItem};

use super::structs::{Path, Visibility};

#[derive(Debug, Clone)]
pub struct TypeAlias {
    name: String,
    vis: Visibility,
    params: Vec<String>,
    module: Path,
}

impl Display for TypeAlias {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {}::{}",
            self.vis.to_string().magenta(),
            "type".green(),
            self.module,
            self.name.yellow(),
        )?;
        if !self.params.is_empty() {
            write!(f, "<{}>", self.params.join(","))?;
        }
        Ok(())
    }
}

impl TreeItem for TypeAlias {
    fn module(&self) -> &Path {
        &self.module
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TreePrintable for TypeAlias {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

impl TypeAlias {
    fn from_syn(item: &syn::ItemType, module: Path) -> Self {
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
}

from_items!(type_aliases_from_items, TypeAlias, Type);
