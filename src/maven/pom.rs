use std::collections::HashMap;

use anyhow::{Context, Result};

use super::coord::MavenCoord;
use super::fetch::MavenFetcher;

#[derive(Debug, Clone)]
pub struct PomDep {
    pub group_id: String,
    pub artifact_id: String,
    pub version: Option<String>,
    pub scope: String,
    pub optional: bool,
    #[allow(dead_code)]
    pub dep_type: String,
    pub exclusions: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct ManagedDep {
    pub group_id: String,
    pub artifact_id: String,
    pub version: String,
    pub scope: Option<String>,
    pub dep_type: String,
    pub exclusions: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct Pom {
    pub parent: Option<MavenCoord>,
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub packaging: String,
    pub properties: HashMap<String, String>,
    pub dependency_management: Vec<ManagedDep>,
    pub dependencies: Vec<PomDep>,
}

/// Parse POM XML into a Pom struct.
pub fn parse_pom(xml: &str) -> Result<Pom> {
    let doc = roxmltree::Document::parse(xml)
        .context("failed to parse POM XML")?;
    let root = doc.root_element();

    let mut pom = Pom {
        packaging: "jar".to_string(),
        ..Default::default()
    };

    for child in root.children().filter(|n| n.is_element()) {
        let tag = child.tag_name().name();
        match tag {
            "parent" => {
                let g = child_text(&child, "groupId").unwrap_or_default();
                let a = child_text(&child, "artifactId").unwrap_or_default();
                let v = child_text(&child, "version").unwrap_or_default();
                if !g.is_empty() && !v.is_empty() {
                    pom.parent = Some(MavenCoord::new(g, a, v));
                }
            }
            "groupId" => pom.group_id = child.text().map(|s| s.trim().to_string()),
            "artifactId" => pom.artifact_id = child.text().map(|s| s.trim().to_string()),
            "version" => pom.version = child.text().map(|s| s.trim().to_string()),
            "packaging" => {
                if let Some(t) = child.text() {
                    pom.packaging = t.trim().to_string();
                }
            }
            "properties" => {
                for prop in child.children().filter(|n| n.is_element()) {
                    if let Some(val) = prop.text() {
                        pom.properties.insert(prop.tag_name().name().to_string(), val.trim().to_string());
                    }
                }
            }
            "dependencyManagement" => {
                if let Some(deps_node) = child.children().find(|n| n.is_element() && n.tag_name().name() == "dependencies") {
                    for dep_node in deps_node.children().filter(|n| n.is_element() && n.tag_name().name() == "dependency") {
                        if let Some(md) = parse_managed_dep(&dep_node) {
                            pom.dependency_management.push(md);
                        }
                    }
                }
            }
            "dependencies" => {
                for dep_node in child.children().filter(|n| n.is_element() && n.tag_name().name() == "dependency") {
                    if let Some(d) = parse_dep(&dep_node) {
                        pom.dependencies.push(d);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(pom)
}

fn child_text(node: &roxmltree::Node, name: &str) -> Option<String> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
        .and_then(|n| n.text())
        .map(|s| s.trim().to_string())
}

fn parse_exclusions(node: &roxmltree::Node) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(excl_node) = node.children().find(|n| n.is_element() && n.tag_name().name() == "exclusions") {
        for e in excl_node.children().filter(|n| n.is_element() && n.tag_name().name() == "exclusion") {
            let g = child_text(&e, "groupId").unwrap_or_default();
            let a = child_text(&e, "artifactId").unwrap_or_default();
            if !g.is_empty() {
                result.push((g, a));
            }
        }
    }
    result
}

fn parse_managed_dep(node: &roxmltree::Node) -> Option<ManagedDep> {
    let group_id = child_text(node, "groupId")?;
    let artifact_id = child_text(node, "artifactId")?;
    let version = child_text(node, "version")?;
    Some(ManagedDep {
        group_id,
        artifact_id,
        version,
        scope: child_text(node, "scope"),
        dep_type: child_text(node, "type").unwrap_or_else(|| "jar".to_string()),
        exclusions: parse_exclusions(node),
    })
}

fn parse_dep(node: &roxmltree::Node) -> Option<PomDep> {
    let group_id = child_text(node, "groupId")?;
    let artifact_id = child_text(node, "artifactId")?;
    Some(PomDep {
        group_id,
        artifact_id,
        version: child_text(node, "version"),
        scope: child_text(node, "scope").unwrap_or_else(|| "compile".to_string()),
        optional: child_text(node, "optional").map(|s| s == "true").unwrap_or(false),
        dep_type: child_text(node, "type").unwrap_or_else(|| "jar".to_string()),
        exclusions: parse_exclusions(node),
    })
}

/// Interpolate `${...}` placeholders in a string using properties map.
pub fn interpolate(s: &str, props: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    // Iterate up to 10 times for recursive interpolation
    for _ in 0..10 {
        let prev = result.clone();
        let mut out = String::with_capacity(result.len());
        let mut rest = result.as_str();
        while let Some(start) = rest.find("${") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            if let Some(end) = after.find('}') {
                let key = &after[..end];
                if let Some(val) = props.get(key) {
                    out.push_str(val);
                } else {
                    // Keep unresolved placeholder
                    out.push_str(&rest[start..start + 2 + end + 1]);
                }
                rest = &after[end + 1..];
            } else {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
        out.push_str(rest);
        result = out;
        if result == prev {
            break;
        }
    }
    result
}

/// Resolve effective POM by walking parent chain, merging properties and depMgmt.
pub fn resolve_effective_pom(
    fetcher: &MavenFetcher,
    coord: &MavenCoord,
    pom_cache: &mut HashMap<MavenCoord, Pom>,
) -> Result<Pom> {
    if let Some(cached) = pom_cache.get(coord) {
        return Ok(cached.clone());
    }

    let xml = fetcher.fetch_pom(coord)?;
    let mut pom = parse_pom(&xml)?;

    // Resolve parent chain
    if let Some(ref parent_coord) = pom.parent.clone() {
        let parent = resolve_effective_pom(fetcher, parent_coord, pom_cache)?;
        merge_parent(&mut pom, &parent);
    }

    // Build properties map including project.* builtins
    if let Some(ref g) = pom.group_id {
        pom.properties.insert("project.groupId".to_string(), g.clone());
        pom.properties.insert("pom.groupId".to_string(), g.clone());
    }
    if let Some(ref v) = pom.version {
        pom.properties.insert("project.version".to_string(), v.clone());
        pom.properties.insert("pom.version".to_string(), v.clone());
    }
    if let Some(ref a) = pom.artifact_id {
        pom.properties.insert("project.artifactId".to_string(), a.clone());
    }

    // Interpolate all strings
    let props = pom.properties.clone();
    interpolate_pom(&mut pom, &props);

    pom_cache.insert(coord.clone(), pom.clone());
    Ok(pom)
}

fn merge_parent(child: &mut Pom, parent: &Pom) {
    if child.group_id.is_none() {
        child.group_id = parent.group_id.clone();
    }
    if child.version.is_none() {
        child.version = parent.version.clone();
    }
    // Merge properties: parent first, child overrides
    for (k, v) in &parent.properties {
        child.properties.entry(k.clone()).or_insert_with(|| v.clone());
    }
    // Merge dependencyManagement: parent entries that aren't overridden by child
    let child_keys: std::collections::HashSet<(String, String)> = child.dependency_management.iter()
        .map(|d| (d.group_id.clone(), d.artifact_id.clone()))
        .collect();
    for md in &parent.dependency_management {
        if !child_keys.contains(&(md.group_id.clone(), md.artifact_id.clone())) {
            child.dependency_management.push(md.clone());
        }
    }
}

fn interpolate_pom(pom: &mut Pom, props: &HashMap<String, String>) {
    if let Some(ref mut g) = pom.group_id {
        *g = interpolate(g, props);
    }
    if let Some(ref mut v) = pom.version {
        *v = interpolate(v, props);
    }
    for md in &mut pom.dependency_management {
        md.group_id = interpolate(&md.group_id, props);
        md.artifact_id = interpolate(&md.artifact_id, props);
        md.version = interpolate(&md.version, props);
    }
    for d in &mut pom.dependencies {
        d.group_id = interpolate(&d.group_id, props);
        d.artifact_id = interpolate(&d.artifact_id, props);
        if let Some(ref mut v) = d.version {
            *v = interpolate(v, props);
        }
        d.scope = interpolate(&d.scope, props);
    }
}
