pub mod attributes;
pub mod deps;
#[allow(non_upper_case_globals)]
pub mod format;
pub mod names;
pub mod positions;
pub mod printer;
pub mod reader;
pub mod trees;

use std::path::Path;

use anyhow::{bail, Result};

use format::MAGIC;
use names::NameTable;
use printer::TastyPrinter;
use reader::TastyReader;

/// Read and dump the contents of a TASTy file.
pub fn dump_tasty(path: &Path) -> Result<()> {
    let data = std::fs::read(path)?;
    if data.len() < 4 {
        bail!("file too small to be a TASTy file");
    }

    let mut r = TastyReader::new(&data);

    // Header magic
    let magic = r.read_bytes(4)?;
    if magic != MAGIC {
        bail!(
            "not a TASTy file: bad magic {:02x}{:02x}{:02x}{:02x}",
            magic[0],
            magic[1],
            magic[2],
            magic[3]
        );
    }

    // Version
    let major = r.read_nat()?;
    let minor = r.read_nat()?;
    let experimental = r.read_nat()?;

    // Tooling version string (Utf8 = Nat length + bytes)
    let tooling_len = r.read_nat()? as usize;
    let tooling = r.read_utf8(tooling_len)?;

    // UUID (16 bytes)
    let uuid = r.read_bytes(16)?;

    // Name table
    let name_table = NameTable::parse(&mut r)?;

    // Sections
    let mut ast_arena = None;
    let mut positions = None;
    let mut attrs = None;

    while !r.at_end() {
        // Section = NameRef + Length + payload
        let section_name_ref = r.read_nat()? as usize;
        let section_len = r.read_nat()? as usize;
        let section_end = r.pos() + section_len;

        let section_name = name_table.display(section_name_ref);

        match section_name.as_str() {
            "ASTs" => {
                let mut sub = r.sub_reader(r.pos(), section_end);
                ast_arena = Some(trees::parse_trees(&mut sub)?);
            }
            "Positions" => {
                let mut sub = r.sub_reader(r.pos(), section_end);
                positions = Some(positions::Positions::parse(&mut sub)?);
            }
            "Attributes" => {
                let mut sub = r.sub_reader(r.pos(), section_end);
                attrs = Some(attributes::Attributes::parse(&mut sub)?);
            }
            _ => {
                // Skip unknown sections (e.g. "Comments")
            }
        }

        r.set_pos(section_end);
    }

    let printer = TastyPrinter {
        major,
        minor,
        experimental,
        tooling,
        uuid,
        names: &name_table,
        trees: ast_arena.as_ref(),
        positions: positions.as_ref(),
        attributes: attrs.as_ref(),
    };

    printer.print()?;
    Ok(())
}
