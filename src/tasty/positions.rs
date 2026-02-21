use anyhow::Result;

use super::reader::TastyReader;

#[derive(Debug)]
pub struct Position {
    pub start: i32,
    pub end: i32,
    pub point: i32,
}

#[derive(Debug)]
pub struct Positions {
    pub entries: Vec<Position>,
}

impl Positions {
    pub fn parse(reader: &mut TastyReader<'_>) -> Result<Self> {
        let mut entries = Vec::new();
        let mut cur_start: i32 = 0;

        while !reader.at_end() {
            let header = reader.read_nat()? as i32;
            if header == 0 {
                // end marker
                break;
            }
            // Delta-encoded: bit 0 = hasEnd, bit 1 = hasPoint
            let has_end = header & 1 != 0;
            let has_point = header & 2 != 0;
            let delta = header >> 2;
            cur_start += delta;

            let end = if has_end {
                cur_start + reader.read_nat()? as i32
            } else {
                cur_start
            };

            let point = if has_point {
                cur_start + reader.read_nat()? as i32
            } else {
                cur_start
            };

            entries.push(Position {
                start: cur_start,
                end,
                point,
            });
        }

        Ok(Positions { entries })
    }
}
