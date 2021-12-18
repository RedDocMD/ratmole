use std::fmt::{self, Display, Formatter};

pub mod consts;
pub mod enums;
pub mod extern_crate;
pub mod module;
pub mod reexport;
pub mod structs;
pub mod types;

#[macro_export]
macro_rules! from_items {
    ($func_name:ident, $type:ty, $item_name:ident) => {
        pub fn $func_name(
            items: &[syn::Item],
            module: &mut crate::item::structs::Path,
        ) -> std::collections::HashMap<Path, Vec<$type>> {
            use std::collections::HashMap;
            use syn::Item;

            let mut things: HashMap<Path, Vec<$type>> = HashMap::new();
            for item in items {
                match item {
                    Item::$item_name(item) => {
                        let s = <$type>::from_syn(item, module.clone());
                        if let Some(existing_things) = things.get_mut(module) {
                            existing_things.push(s);
                        } else {
                            things.insert(module.clone(), vec![s]);
                        }
                    }
                    Item::Mod(item) => {
                        module.push_name(item.ident.to_string());
                        if let Some((_, content)) = &item.content {
                            let new_things = $func_name(content, module);
                            things.extend(new_things);
                        }
                        module.pop();
                    }
                    _ => {}
                }
            }
            things
        }
    };
}

pub enum Item {
    Struct(structs::Struct),
    Enum(enums::Enum),
    Const(consts::Const),
    TypeAlias(types::TypeAlias),
    Module(module::Module),
    ReExport(reexport::ReExport),
}

impl Display for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Item::Struct(s) => write!(f, "{}", s),
            Item::Module(m) => write!(f, "{}", m),
            Item::Enum(e) => write!(f, "{}", e),
            Item::Const(c) => write!(f, "{}", c),
            Item::TypeAlias(ta) => write!(f, "{}", ta),
            Item::ReExport(r) => write!(f, "{}", r),
        }
    }
}
