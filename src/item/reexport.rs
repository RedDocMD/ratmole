use std::fmt::{self, Display, Formatter};

use crate::{printer::TreePrintable, tree::TreeItem, use_path::UsePath};

use super::{structs::Path, Item};

pub struct ReExport {
    module: Path,
    use_path: UsePath,
    items: Vec<Item>,
    name: String,
}

impl ReExport {
    pub fn new(module: Path, use_path: UsePath, items: Vec<Item>) -> Self {
        let name = use_path.to_string();
        Self {
            module,
            use_path,
            items,
            name,
        }
    }
}

impl Display for ReExport {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let items_str: Vec<_> = self.items.iter().map(Item::to_string).collect();
        write!(f, "{} => [{}]", self.use_path, items_str.join(", "))
    }
}

impl TreeItem for ReExport {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &Path {
        &self.module
    }
}

impl TreePrintable for ReExport {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}
