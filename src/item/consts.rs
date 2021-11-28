use crate::from_items;
use crate::printer::TreePrintable;
use crate::tree::TreeItem;

use super::structs::{Path, Visibility};
use colored::*;

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
            "{}{} {}::{}",
            self.vis.to_string().magenta(),
            "const".green(),
            self.module,
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

from_items!(consts_from_items, Const, Const);
