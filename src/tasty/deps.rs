use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use xxhash_rust::xxh3::Xxh3;

use super::attributes::Attributes;
use super::format::{self, MAGIC};
use super::names::NameTable;
use super::reader::TastyReader;
use super::trees::{self, TreeArena, TreeNode};

/// Information extracted from a single .tasty file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TastyFileInfo {
    /// Path relative to classes_dir (e.g. "Data.tasty")
    pub tasty_path: String,
    /// Source file path relative to project root (from SOURCEFILEattr)
    pub source_file: String,
    /// Hash of public API signatures
    pub api_hash: u64,
    /// Set of referenced user class names (simple or qualified, excluding scala.*/java.*)
    pub deps: HashSet<String>,
}

/// Full incremental compilation state, serialized to .sb/cache/incr-state.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalState {
    pub source_hashes: HashMap<String, u64>,
    pub tasty_files: Vec<TastyFileInfo>,
    pub dep_hash: String,
}

impl IncrementalState {
    pub fn load(project_root: &Path) -> Option<Self> {
        let path = project_root.join(".sb/cache/incr-state.json");
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, project_root: &Path) -> Result<()> {
        let dir = project_root.join(".sb/cache");
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string(self)?;
        std::fs::write(dir.join("incr-state.json"), json)?;
        Ok(())
    }

    /// Build a reverse dependency map: source_file → set of source files that depend on it.
    pub fn reverse_dep_map(&self) -> HashMap<String, HashSet<String>> {
        // First build: class_name → source_file
        let mut class_to_source: HashMap<String, String> = HashMap::new();
        for info in &self.tasty_files {
            // Derive class name from tasty_path: "foo/Bar.tasty" → "foo.Bar"
            let class_name = tasty_path_to_class_name(&info.tasty_path);
            class_to_source.insert(class_name, info.source_file.clone());
        }

        // Build: source_file → set of source files that depend on it
        let mut rev_deps: HashMap<String, HashSet<String>> = HashMap::new();
        for info in &self.tasty_files {
            for dep_class in &info.deps {
                if let Some(dep_source) = class_to_source.get(dep_class) {
                    if dep_source != &info.source_file {
                        rev_deps
                            .entry(dep_source.clone())
                            .or_default()
                            .insert(info.source_file.clone());
                    }
                }
            }
        }
        rev_deps
    }

    /// Get API hash for a source file.
    pub fn api_hash_for_source(&self, source: &str) -> Option<u64> {
        // A source file may produce multiple tasty files; combine their API hashes
        let mut hasher = Xxh3::new();
        let mut found = false;
        for info in &self.tasty_files {
            if info.source_file == source {
                hasher.update(&info.api_hash.to_le_bytes());
                found = true;
            }
        }
        if found {
            Some(hasher.digest())
        } else {
            None
        }
    }
}

fn tasty_path_to_class_name(tasty_path: &str) -> String {
    tasty_path
        .trim_end_matches(".tasty")
        .replace('/', ".")
        .replace('\\', ".")
}

/// Scan all .tasty files in a classes directory and extract dependency info.
pub fn scan_classes_dir(classes_dir: &Path, project_root: &Path) -> Result<Vec<TastyFileInfo>> {
    let mut results = Vec::new();

    let tasty_files: Vec<_> = WalkDir::new(classes_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().is_some_and(|ext| ext == "tasty")
        })
        .map(|e| e.into_path())
        .collect();

    for tasty_file in tasty_files {
        match extract_deps(&tasty_file, classes_dir, project_root) {
            Ok(info) => results.push(info),
            Err(e) => {
                eprintln!(
                    "warning: failed to extract deps from {}: {}",
                    tasty_file.display(),
                    e
                );
            }
        }
    }

    Ok(results)
}

/// Extract dependency information from a single .tasty file.
fn extract_deps(
    tasty_path: &Path,
    classes_dir: &Path,
    project_root: &Path,
) -> Result<TastyFileInfo> {
    let data = std::fs::read(tasty_path)?;
    let mut r = TastyReader::new(&data);

    // Header
    let magic = r.read_bytes(4)?;
    if magic != MAGIC {
        anyhow::bail!("not a TASTy file");
    }
    let _major = r.read_nat()?;
    let _minor = r.read_nat()?;
    let _experimental = r.read_nat()?;
    let tooling_len = r.read_nat()? as usize;
    let _tooling = r.read_utf8(tooling_len)?;
    let _uuid = r.read_bytes(16)?;

    // Name table
    let name_table = NameTable::parse(&mut r)?;

    // Sections
    let mut ast_arena = None;
    let mut attrs = None;

    while !r.at_end() {
        let section_name_ref = r.read_nat()? as usize;
        let section_len = r.read_nat()? as usize;
        let section_end = r.pos() + section_len;
        let section_name = name_table.display(section_name_ref);

        match section_name.as_str() {
            "ASTs" => {
                let mut sub = r.sub_reader(r.pos(), section_end);
                ast_arena = Some(trees::parse_trees(&mut sub)?);
            }
            "Attributes" => {
                let mut sub = r.sub_reader(r.pos(), section_end);
                attrs = Some(Attributes::parse(&mut sub)?);
            }
            _ => {}
        }
        r.set_pos(section_end);
    }

    // Extract source file from attributes
    let source_file = extract_source_file(&attrs, &name_table, tasty_path, project_root);

    // Extract deps and API hash from AST
    let (deps, api_hash) = if let Some(ref arena) = ast_arena {
        let deps = extract_type_refs(arena, &name_table);
        let api_hash = compute_api_hash(arena, &name_table);
        (deps, api_hash)
    } else {
        (HashSet::new(), 0)
    };

    let rel_tasty = tasty_path
        .strip_prefix(classes_dir)
        .unwrap_or(tasty_path)
        .to_string_lossy()
        .to_string();

    Ok(TastyFileInfo {
        tasty_path: rel_tasty,
        source_file,
        api_hash,
        deps,
    })
}

fn extract_source_file(
    attrs: &Option<Attributes>,
    name_table: &NameTable<'_>,
    tasty_path: &Path,
    project_root: &Path,
) -> String {
    // SOURCEFILEattr has tag 129
    if let Some(attrs) = attrs {
        for &(tag, name_ref) in &attrs.utf8ref_attrs {
            if tag == 129 {
                let source = name_table.display(name_ref as usize);
                // The source file attr is typically a relative path from the project
                // Try to normalize it
                let source_path = Path::new(&source);
                if source_path.is_absolute() {
                    return source_path
                        .strip_prefix(project_root)
                        .unwrap_or(source_path)
                        .to_string_lossy()
                        .to_string();
                }
                return source;
            }
        }
    }
    // Fallback: derive from tasty path
    let stem = tasty_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy();
    format!("src/main/scala/{stem}.scala")
}

/// Walk all tree nodes and collect TYPEREF/TERMREF name references that look like user types.
fn extract_type_refs(arena: &TreeArena, name_table: &NameTable<'_>) -> HashSet<String> {
    let mut deps = HashSet::new();

    for node in &arena.nodes {
        match node.tag {
            // Cat4: TERMREF(115), TYPEREF(117) — nat = name ref, child = qualifier type
            format::TERMREF | format::TYPEREF => {
                if let Some(name_ref) = node.nat {
                    let name = name_table.display(name_ref as usize);
                    if !is_stdlib_name(&name) && !name.is_empty() {
                        // Get the simple name; we'll resolve against the qualifier
                        deps.insert(name);
                    }
                }
            }
            // Cat5: TERMREFin(174), TYPEREFin(175), SELECTin(176) — nat = name ref
            format::TERMREFin | format::TYPEREFin | format::SELECTin => {
                if let Some(name_ref) = node.nat {
                    let name = name_table.display(name_ref as usize);
                    if !is_stdlib_name(&name) && !name.is_empty() {
                        deps.insert(name);
                    }
                }
            }
            _ => {}
        }
    }

    deps
}

fn is_stdlib_name(name: &str) -> bool {
    name.starts_with("scala")
        || name.starts_with("java")
        || name.starts_with("dotty")
        || name == "<init>"
        || name == "<empty>"
        || name == "_root_"
        || name == "<repeated>"
}

/// Compute a hash of the public API: walk top-level VALDEF/DEFDEF/TYPEDEF that are not PRIVATE.
fn compute_api_hash(arena: &TreeArena, name_table: &NameTable<'_>) -> u64 {
    let mut hasher = Xxh3::new();

    // Walk all nodes; for top-level defs, hash if not private
    for node in &arena.nodes {
        match node.tag {
            format::VALDEF | format::DEFDEF | format::TYPEDEF => {
                if !has_private_modifier(node, arena) {
                    // Hash tag + name + children structure
                    hasher.update(&[node.tag]);
                    if let Some(name_ref) = node.nat {
                        let name = name_table.display(name_ref as usize);
                        hasher.update(name.as_bytes());
                    }
                    // Hash the type subtree structure
                    hash_children_structure(node, arena, &mut hasher);
                }
            }
            _ => {}
        }
    }

    hasher.digest()
}

fn has_private_modifier(node: &TreeNode, arena: &TreeArena) -> bool {
    for &child_id in &node.children {
        let child = arena.get(child_id);
        if child.tag == format::PRIVATE {
            return true;
        }
    }
    false
}

fn hash_children_structure(node: &TreeNode, arena: &TreeArena, hasher: &mut Xxh3) {
    for &child_id in &node.children {
        let child = arena.get(child_id);
        hasher.update(&[child.tag]);
        if let Some(n) = child.nat {
            hasher.update(&n.to_le_bytes());
        }
        // Recurse into type trees but not deep into body expressions
        match child.tag {
            // Type-related tags: recurse
            format::APPLIEDtype | format::APPLIEDtpt | format::ANDtype | format::ORtype
            | format::TYPEBOUNDS | format::TYPEBOUNDStpt | format::POLYtype
            | format::METHODtype | format::TYPELAMBDAtype | format::REFINEDtype
            | format::REFINEDtpt | format::BYNAMEtype | format::BYNAMEtpt
            | format::ANNOTATEDtype | format::ANNOTATEDtpt | format::PARAM
            | format::TYPEPARAM | format::TYPEREF | format::TERMREF | format::TYPEREFin
            | format::TERMREFin | format::SELECTin => {
                hash_children_structure(child, arena, hasher);
            }
            _ => {}
        }
    }
}
