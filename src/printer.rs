use std::fmt::{self, Formatter};

pub trait TreePrintable {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result;
    fn children(&self) -> Vec<&dyn TreePrintable>;

    fn tree_print(&self, f: &mut Formatter<'_>) -> fmt::Result
    where
        Self: Sized,
    {
        rec_tree_print(self, f, &mut vec![DepthPosition::Root])
    }
}

#[derive(PartialEq, Eq)]
enum DepthPosition {
    Root,
    Last,
    Other,
}

fn rec_tree_print(
    node: &dyn TreePrintable,
    f: &mut Formatter<'_>,
    positions: &mut Vec<DepthPosition>,
) -> fmt::Result {
    for pos in &positions[0..positions.len() - 1] {
        match pos {
            DepthPosition::Other => write!(f, "\u{2502}   ")?,
            DepthPosition::Last => write!(f, "    ")?,
            DepthPosition::Root => {}
        }
    }
    match positions.last().unwrap() {
        DepthPosition::Root => { /* Do Nothing */ }
        DepthPosition::Last => write!(f, "\u{2514}\u{2500}\u{2500} ")?,
        DepthPosition::Other => write!(f, "\u{251C}\u{2500}\u{2500} ")?,
    }
    node.single_write(f)?;
    writeln!(f)?;
    let children = node.children();
    for (idx, new_node) in children.iter().enumerate() {
        let new_pos = if idx == children.len() - 1 {
            DepthPosition::Last
        } else {
            DepthPosition::Other
        };
        positions.push(new_pos);
        rec_tree_print(*new_node, f, positions)?;
        positions.pop();
    }
    Ok(())
}
