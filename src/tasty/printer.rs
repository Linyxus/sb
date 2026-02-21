use std::io::{self, BufWriter, Write};

use super::attributes::Attributes;
use super::format::tag_name;
use super::names::NameTable;
use super::positions::Positions;
use super::trees::{NodeId, TreeArena};

pub struct TastyPrinter<'a> {
    pub major: u64,
    pub minor: u64,
    pub experimental: u64,
    pub tooling: &'a str,
    pub uuid: &'a [u8],
    pub names: &'a NameTable<'a>,
    pub trees: Option<&'a TreeArena>,
    pub positions: Option<&'a Positions>,
    pub attributes: Option<&'a Attributes>,
}

impl<'a> TastyPrinter<'a> {
    pub fn print(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut w = BufWriter::new(stdout.lock());

        writeln!(w, "TASTy file")?;
        writeln!(
            w,
            "  version: {}.{}.{}",
            self.major, self.minor, self.experimental
        )?;
        writeln!(w, "  tooling: {}", self.tooling)?;
        write!(w, "  uuid: ")?;
        for b in self.uuid {
            write!(w, "{:02x}", b)?;
        }
        writeln!(w)?;
        writeln!(w)?;

        // Name table
        writeln!(w, "Names ({} entries):", self.names.entries.len())?;
        for (i, _entry) in self.names.entries.iter().enumerate() {
            writeln!(w, "  [{}]: {}", i, self.names.display(i))?;
        }
        writeln!(w)?;

        // AST
        if let Some(trees) = self.trees {
            writeln!(w, "Trees ({} nodes):", trees.nodes.len())?;
            // Print top-level nodes (find roots: nodes not referenced as children)
            // Simple approach: just print from first node
            if !trees.nodes.is_empty() {
                // Find all root-like nodes by iterating and printing top-level
                let mut printed = vec![false; trees.nodes.len()];
                self.mark_children(trees, &mut printed);
                for i in 0..trees.nodes.len() {
                    if !printed[i] {
                        self.print_tree(&mut w, trees, NodeId(i as u32), 1)?;
                    }
                }
            }
            writeln!(w)?;
        }

        // Attributes
        if let Some(attrs) = self.attributes {
            writeln!(w, "Attributes:")?;
            for tag in &attrs.boolean_attrs {
                writeln!(w, "  {}", Attributes::attr_name(*tag))?;
            }
            for (tag, name_ref) in &attrs.utf8ref_attrs {
                let val = self.names.display(*name_ref as usize);
                writeln!(w, "  {} = \"{}\"", Attributes::attr_name(*tag), val)?;
            }
            writeln!(w)?;
        }

        // Positions
        if let Some(positions) = self.positions {
            writeln!(w, "Positions ({} entries)", positions.entries.len())?;
        }

        w.flush()
    }

    fn mark_children(&self, arena: &TreeArena, printed: &mut [bool]) {
        for node in &arena.nodes {
            for child in &node.children {
                printed[child.0 as usize] = true;
            }
        }
    }

    fn print_tree(
        &self,
        w: &mut impl Write,
        arena: &TreeArena,
        id: NodeId,
        indent: usize,
    ) -> io::Result<()> {
        let node = arena.get(id);
        let prefix = "  ".repeat(indent);
        let name = tag_name(node.tag);

        match node.nat {
            Some(n) => {
                // Try to resolve as name ref for relevant tags
                let name_str = self.names.display(n as usize);
                writeln!(w, "{}{} {}[={}]", prefix, name, n, name_str)?;
            }
            None => {
                writeln!(w, "{}{}", prefix, name)?;
            }
        }

        if node.binder_param_names.is_empty() {
            for child in &node.children {
                self.print_tree(w, arena, *child, indent + 1)?;
            }
        } else {
            // Binder type: children[0] = result, children[1..] paired with param names
            if let Some(result) = node.children.first() {
                self.print_tree(w, arena, *result, indent + 1)?;
            }
            for (i, name_ref) in node.binder_param_names.iter().enumerate() {
                let child_idx = i + 1;
                if child_idx < node.children.len() {
                    let pname = self.names.display(*name_ref as usize);
                    writeln!(w, "{}  param {}:", prefix, pname)?;
                    self.print_tree(w, arena, node.children[child_idx], indent + 2)?;
                }
            }
        }
        Ok(())
    }
}
