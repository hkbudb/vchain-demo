use super::{IdType, Parameter, RawObject, SetElementType, SkipLstLvlType};
use crate::acc::{
    self,
    curve::{G1Affine, G2Affine},
    Accumulator,
};
use crate::set::MultiSet;
use anyhow::{Context, Error, Result};
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

#[inline]
pub fn multiset_to_g1(set: &MultiSet<SetElementType>, param: &Parameter) -> G1Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g1_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g1(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g1_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g1(&set),
    }
}

#[inline]
pub fn multiset_to_g2(set: &MultiSet<SetElementType>, param: &Parameter) -> G2Affine {
    match (param.acc_type, param.use_sk) {
        (acc::Type::ACC1, true) => acc::Acc1::cal_acc_g2_sk(&set),
        (acc::Type::ACC1, false) => acc::Acc1::cal_acc_g2(&set),
        (acc::Type::ACC2, true) => acc::Acc2::cal_acc_g2_sk(&set),
        (acc::Type::ACC2, false) => acc::Acc2::cal_acc_g2(&set),
    }
}

#[inline]
pub fn skipped_blocks_num(level: SkipLstLvlType) -> IdType {
    1 << (level + 2)
}

// input format: block_id sep [ v_data ] sep { w_data }
// sep = \t or space
// v_data = v_1 comma v_2 ...
// w_data = w_1 comma w_2 ...
pub fn load_raw_obj_from_file(path: &Path) -> Result<BTreeMap<IdType, Vec<RawObject>>> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;
    load_raw_obj_from_str(&buf)
}
pub fn load_raw_obj_from_str(input: &str) -> Result<BTreeMap<IdType, Vec<RawObject>>> {
    let mut res = BTreeMap::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut split_str = line.splitn(3, |c| c == '[' || c == ']');
        let block_id: IdType = split_str
            .next()
            .context(format!("failed to parse line {}", line))?
            .trim()
            .parse()?;
        let v_data: Vec<u32> = split_str
            .next()
            .context(format!("failed to parse line {}", line))?
            .trim()
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.parse::<u32>().map_err(Error::from))
            .collect::<Result<_>>()?;
        let w_data: HashSet<String> = split_str
            .next()
            .context(format!("failed to parse line {}", line))?
            .trim()
            .replace('{', "")
            .replace('}', "")
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();

        let raw_obj = RawObject {
            block_id,
            v_data,
            w_data,
        };
        res.entry(block_id).or_insert_with(Vec::new).push(raw_obj);
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_raw_obj() {
        let input = "1\t[1,2]\t{a,b}\n2 [ 3, 4 ] { c, d, }\n2\t[ 5, 6 ]\t { e }\n";
        let expect = {
            let mut out: BTreeMap<IdType, Vec<RawObject>> = BTreeMap::new();
            out.insert(
                1,
                vec![RawObject {
                    block_id: 1,
                    v_data: vec![1, 2],
                    w_data: ["a".to_owned(), "b".to_owned()].iter().cloned().collect(),
                }],
            );
            out.insert(
                2,
                vec![
                    RawObject {
                        block_id: 2,
                        v_data: vec![3, 4],
                        w_data: ["c".to_owned(), "d".to_owned()].iter().cloned().collect(),
                    },
                    RawObject {
                        block_id: 2,
                        v_data: vec![5, 6],
                        w_data: ["e".to_owned()].iter().cloned().collect(),
                    },
                ],
            );
            out
        };
        assert_eq!(load_raw_obj_from_str(&input).unwrap(), expect);
    }
}
