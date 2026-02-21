use anyhow::Result;

use super::reader::TastyReader;

#[derive(Debug)]
pub struct Attributes {
    pub boolean_attrs: Vec<u8>,
    /// String attributes: (tag, Utf8Ref into name table)
    pub utf8ref_attrs: Vec<(u8, u32)>,
}

impl Attributes {
    pub fn parse(reader: &mut TastyReader<'_>) -> Result<Self> {
        let mut boolean_attrs = Vec::new();
        let mut utf8ref_attrs = Vec::new();

        while !reader.at_end() {
            let tag = reader.read_byte()?;
            if tag <= 32 {
                // Attribute Category 1 (tags 1-32): boolean, tag only
                boolean_attrs.push(tag);
            } else if tag >= 129 && tag <= 160 {
                // Attribute Category 3 (tags 129-160): tag + Utf8Ref
                let name_ref = reader.read_nat()? as u32;
                utf8ref_attrs.push((tag, name_ref));
            }
            // Categories 2 (33-128) and 4 (161-255) are unassigned
        }

        Ok(Attributes {
            boolean_attrs,
            utf8ref_attrs,
        })
    }

    pub fn attr_name(tag: u8) -> &'static str {
        match tag {
            1 => "SCALA2STANDARDLIBRARYattr",
            2 => "EXPLICITNULLSattr",
            3 => "CAPTURECHECKEDattr",
            4 => "WITHPUREFUNSattr",
            5 => "JAVAattr",
            6 => "OUTLINEattr",
            129 => "SOURCEFILEattr",
            _ => "UNKNOWN",
        }
    }
}
