use std::collections::HashMap;

use anyhow::{bail, Result};

use super::format::{self, ast_category, SHAREDterm, SHAREDtype};
use super::reader::TastyReader;

/// Index into the tree arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeId(pub u32);

#[derive(Debug)]
pub struct TreeNode {
    pub tag: u8,
    /// Nat payload for cat2/cat4 nodes, or first leading nat for cat5 nodes.
    pub nat: Option<u64>,
    /// Additional leading nats for cat5 nodes (second nat for PARAMtype).
    pub nat2: Option<u64>,
    /// Child node indices.
    pub children: Vec<NodeId>,
    /// For binder types (POLYtype, METHODtype, TYPELAMBDAtype): parameter name refs
    /// interleaved with child type trees. children[0] = result type,
    /// children[1..] = param types, binder_param_names[i] = name for children[i+1].
    pub binder_param_names: Vec<u64>,
}

/// Flat arena of tree nodes.
pub struct TreeArena {
    pub nodes: Vec<TreeNode>,
}

impl TreeArena {
    fn alloc(&mut self, node: TreeNode) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: NodeId) -> &TreeNode {
        &self.nodes[id.0 as usize]
    }
}

/// Returns the number of leading reference nats in a cat5 node payload.
/// Positive = that many leading nats. Negative = binder type with interleaved trees + nats.
fn num_refs(tag: u8) -> i32 {
    match tag {
        format::VALDEF | format::DEFDEF | format::TYPEDEF | format::TYPEPARAM
        | format::PARAM | format::NAMEDARG | format::RETURN | format::BIND
        | format::SELFDEF | format::REFINEDtype | format::TERMREFin
        | format::TYPEREFin | format::SELECTin | format::HOLE => 1,
        format::PARAMtype => 2,
        format::POLYtype | format::TYPELAMBDAtype | format::METHODtype => -1,
        _ => 0,
    }
}

fn is_binder(tag: u8) -> bool {
    matches!(
        tag,
        format::POLYtype | format::TYPELAMBDAtype | format::METHODtype
    )
}

/// Parse the AST section from a TASTy file.
pub fn parse_trees(reader: &mut TastyReader<'_>) -> Result<TreeArena> {
    let base = reader.pos();
    let mut arena = TreeArena { nodes: Vec::new() };
    let mut shared: HashMap<u32, NodeId> = HashMap::new();

    while !reader.at_end() {
        parse_tree(reader, base, &mut arena, &mut shared)?;
    }

    Ok(arena)
}

fn parse_tree(
    reader: &mut TastyReader<'_>,
    base: usize,
    arena: &mut TreeArena,
    shared: &mut HashMap<u32, NodeId>,
) -> Result<NodeId> {
    let rel_pos = (reader.pos() - base) as u32;
    let tag = reader.read_byte()?;
    let cat = ast_category(tag);

    // SHARED references
    if tag == SHAREDterm || tag == SHAREDtype {
        let offset = reader.read_nat()? as u32;
        if let Some(&id) = shared.get(&offset) {
            // Register the SHARED itself so later SHAREDs can chain through it
            shared.insert(rel_pos, id);
            return Ok(id);
        }
        bail!(
            "SHARED reference to unknown offset {} at rel_pos {}",
            offset,
            rel_pos
        );
    }

    let node_id = match cat {
        1 => arena.alloc(TreeNode {
            tag,
            nat: None,
            nat2: None,
            children: Vec::new(),
            binder_param_names: Vec::new(),
        }),
        2 => {
            let n = reader.read_nat()?;
            arena.alloc(TreeNode {
                tag,
                nat: Some(n),
                nat2: None,
                children: Vec::new(),
                binder_param_names: Vec::new(),
            })
        }
        3 => {
            let child = parse_tree(reader, base, arena, shared)?;
            arena.alloc(TreeNode {
                tag,
                nat: None,
                nat2: None,
                children: vec![child],
                binder_param_names: Vec::new(),
            })
        }
        4 => {
            let n = reader.read_nat()?;
            let child = parse_tree(reader, base, arena, shared)?;
            arena.alloc(TreeNode {
                tag,
                nat: Some(n),
                nat2: None,
                children: vec![child],
                binder_param_names: Vec::new(),
            })
        }
        5 => {
            let len = reader.read_nat()? as usize;
            let end = reader.pos() + len;
            let refs = num_refs(tag);

            let mut nat1 = None;
            let mut nat2 = None;

            if refs >= 1 {
                nat1 = Some(reader.read_nat()?);
            }
            if refs >= 2 {
                nat2 = Some(reader.read_nat()?);
            }

            let mut children = Vec::new();
            let mut binder_param_names = Vec::new();

            if is_binder(tag) {
                // Binder types: result_Type tree, then (typeOrBounds tree + paramName nat) pairs
                if reader.pos() < end {
                    let result_child = parse_tree(reader, base, arena, shared)?;
                    children.push(result_child);
                }
                while reader.pos() < end {
                    let type_child = parse_tree(reader, base, arena, shared)?;
                    children.push(type_child);
                    if reader.pos() < end {
                        let name_ref = reader.read_nat()?;
                        binder_param_names.push(name_ref);
                    }
                }
            } else {
                while reader.pos() < end {
                    let child = parse_tree(reader, base, arena, shared)?;
                    children.push(child);
                }
            }

            arena.alloc(TreeNode {
                tag,
                nat: nat1,
                nat2: nat2,
                children,
                binder_param_names,
            })
        }
        _ => bail!(
            "unknown tag {} (category {}) at offset {}",
            tag,
            cat,
            rel_pos
        ),
    };

    shared.insert(rel_pos, node_id);
    Ok(node_id)
}
