use anyhow::{bail, Result};

use super::reader::TastyReader;

/// Name tag constants from TastyFormat.scala
pub const UTF8: u8 = 1;
pub const QUALIFIED: u8 = 2;
pub const EXPANDED: u8 = 3;
pub const EXPANDPREFIX: u8 = 4;
pub const UNIQUE: u8 = 10;
pub const DEFAULTGETTER: u8 = 11;
pub const SUPERACCESSOR: u8 = 20;
pub const INLINEACCESSOR: u8 = 21;
pub const BODYRETAINER: u8 = 22;
pub const OBJECTCLASS: u8 = 23;
pub const SIGNED: u8 = 63;
pub const TARGETSIGNED: u8 = 62;

/// A reference to a name in the name table.
pub type NameRef = u32;

#[derive(Debug)]
pub enum NameEntry<'a> {
    Utf8(&'a str),
    Qualified(NameRef, NameRef),
    Expanded(NameRef, NameRef),
    ExpandPrefix(NameRef, NameRef),
    /// Unique(separator, num, underlying?)
    Unique(NameRef, u32, Option<NameRef>),
    DefaultGetter(NameRef, u32),
    SuperAccessor(NameRef),
    InlineAccessor(NameRef),
    BodyRetainer(NameRef),
    ObjectClass(NameRef),
    /// Signed(original, result, paramSigs) — paramSigs can be negative (type param section lengths)
    Signed(NameRef, NameRef, Vec<i64>),
    /// TargetSigned(original, target, result, paramSigs)
    TargetSigned(NameRef, NameRef, NameRef, Vec<i64>),
}

pub struct NameTable<'a> {
    pub entries: Vec<NameEntry<'a>>,
}

impl<'a> NameTable<'a> {
    pub fn parse(reader: &mut TastyReader<'a>) -> Result<Self> {
        let table_len = reader.read_nat()? as usize;
        let table_end = reader.pos() + table_len;
        let mut entries = Vec::new();

        while reader.pos() < table_end {
            let tag = reader.read_byte()?;
            let entry = match tag {
                UTF8 => {
                    let len = reader.read_nat()? as usize;
                    let s = reader.read_utf8(len)?;
                    NameEntry::Utf8(s)
                }
                QUALIFIED | EXPANDED | EXPANDPREFIX => {
                    let _len = reader.read_nat()?;
                    let qual = reader.read_nat()? as NameRef;
                    let name = reader.read_nat()? as NameRef;
                    match tag {
                        QUALIFIED => NameEntry::Qualified(qual, name),
                        EXPANDED => NameEntry::Expanded(qual, name),
                        _ => NameEntry::ExpandPrefix(qual, name),
                    }
                }
                UNIQUE => {
                    // UNIQUE Length separator_NameRef uniqid_Nat underlying_NameRef?
                    let len = reader.read_nat()? as usize;
                    let end = reader.pos() + len;
                    let separator = reader.read_nat()? as NameRef;
                    let num = reader.read_nat()? as u32;
                    let underlying = if reader.pos() < end {
                        Some(reader.read_nat()? as NameRef)
                    } else {
                        None
                    };
                    NameEntry::Unique(separator, num, underlying)
                }
                DEFAULTGETTER => {
                    let _len = reader.read_nat()?;
                    let name = reader.read_nat()? as NameRef;
                    let idx = reader.read_nat()? as u32;
                    NameEntry::DefaultGetter(name, idx)
                }
                SUPERACCESSOR => {
                    let _len = reader.read_nat()?;
                    let name = reader.read_nat()? as NameRef;
                    NameEntry::SuperAccessor(name)
                }
                INLINEACCESSOR => {
                    let _len = reader.read_nat()?;
                    let name = reader.read_nat()? as NameRef;
                    NameEntry::InlineAccessor(name)
                }
                BODYRETAINER => {
                    let _len = reader.read_nat()?;
                    let name = reader.read_nat()? as NameRef;
                    NameEntry::BodyRetainer(name)
                }
                OBJECTCLASS => {
                    let _len = reader.read_nat()?;
                    let name = reader.read_nat()? as NameRef;
                    NameEntry::ObjectClass(name)
                }
                SIGNED => {
                    // SIGNED Length original_NameRef resultSig_NameRef ParamSig*
                    let len = reader.read_nat()? as usize;
                    let end = reader.pos() + len;
                    let original = reader.read_nat()? as NameRef;
                    let result = reader.read_nat()? as NameRef;
                    let mut params = Vec::new();
                    while reader.pos() < end {
                        params.push(reader.read_int()?);
                    }
                    NameEntry::Signed(original, result, params)
                }
                TARGETSIGNED => {
                    // TARGETSIGNED Length original_NameRef target_NameRef resultSig_NameRef ParamSig*
                    let len = reader.read_nat()? as usize;
                    let end = reader.pos() + len;
                    let original = reader.read_nat()? as NameRef;
                    let target = reader.read_nat()? as NameRef;
                    let result = reader.read_nat()? as NameRef;
                    let mut params = Vec::new();
                    while reader.pos() < end {
                        params.push(reader.read_int()?);
                    }
                    NameEntry::TargetSigned(original, target, result, params)
                }
                _ => bail!("unknown name tag {} at offset {}", tag, reader.pos()),
            };
            entries.push(entry);
        }

        Ok(NameTable { entries })
    }

    pub fn display(&self, idx: usize) -> String {
        if idx >= self.entries.len() {
            return format!("<invalid name ref {}>", idx);
        }
        match &self.entries[idx] {
            NameEntry::Utf8(s) => s.to_string(),
            NameEntry::Qualified(q, n) => {
                format!("{}.{}", self.display(*q as usize), self.display(*n as usize))
            }
            NameEntry::Expanded(q, n) => {
                format!("{}$$${}", self.display(*q as usize), self.display(*n as usize))
            }
            NameEntry::ExpandPrefix(q, n) => {
                format!("{}$${}", self.display(*q as usize), self.display(*n as usize))
            }
            NameEntry::Unique(sep, num, underlying) => {
                let base = underlying
                    .map(|u| self.display(u as usize))
                    .unwrap_or_default();
                let sep_str = self.display(*sep as usize);
                format!("{}{}{}", base, sep_str, num)
            }
            NameEntry::DefaultGetter(name, idx) => {
                format!("{}$default${}", self.display(*name as usize), idx)
            }
            NameEntry::SuperAccessor(name) => {
                format!("super${}", self.display(*name as usize))
            }
            NameEntry::InlineAccessor(name) => {
                format!("inline${}", self.display(*name as usize))
            }
            NameEntry::BodyRetainer(name) => {
                format!("bodyretainer${}", self.display(*name as usize))
            }
            NameEntry::ObjectClass(name) => format!("{}$", self.display(*name as usize)),
            NameEntry::Signed(orig, result, params) => {
                let params_str = format_param_sigs(self, params);
                format!(
                    "{}({}): {}",
                    self.display(*orig as usize),
                    params_str,
                    self.display(*result as usize)
                )
            }
            NameEntry::TargetSigned(orig, _target, result, params) => {
                let params_str = format_param_sigs(self, params);
                format!(
                    "@target {}({}): {}",
                    self.display(*orig as usize),
                    params_str,
                    self.display(*result as usize)
                )
            }
        }
    }
}

fn format_param_sigs(table: &NameTable<'_>, params: &[i64]) -> String {
    params
        .iter()
        .map(|&p| {
            if p < 0 {
                format!("[{}]", -p)
            } else {
                table.display(p as usize)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
