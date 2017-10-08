use csv;
use std::collections::HashMap;
use std::fs::File;


#[derive(Debug,Deserialize)]
pub struct Operation {
    code: u8,
    mnemonic: String,
    operand1: Option<String>,
    operand2: Option<String>,
    bytes: u8,
    flag_z: Option<char>,
    flag_h: Option<char>,
    flag_n: Option<char>,
    flag_c: Option<char>,
    cycles_ok: u8,
    cycles_no: Option<u8>
}

impl Operation {

}

pub struct Ops {
    ops: HashMap<u8, Operation>,
    cb_ops: HashMap<u8, Operation>
}

impl Ops {
    pub fn new() -> Ops { Ops { ops: HashMap::new(), cb_ops: HashMap::new() } }

    pub fn load_ops(&mut self) {
        Ops::load_op_type(&mut self.ops, "data/unprefixed.csv");
        Ops::load_op_type(&mut self.cb_ops, "data/cbprefixed.csv");
    }

    pub fn load_op_type(map: &mut HashMap<u8, Operation>, filepath: &str) {
        let file = match File::open(filepath) {
            Ok(f) => { f },
            Err(_) => { panic!("File not found: {}", filepath) }
        };

        let mut rdr = csv::Reader::from_reader(file);

        for result in rdr.deserialize() {
            let op: Operation = match result {
                Ok(r) => { r },
                Err(_) => { panic!("Opcodes CSV file is broken! {}", filepath) }
            };
            map.insert(op.code, op);
        }
    }
}

/// returns an error with the amount of extra bytes required to decode the operation
pub fn decode_op(op: u16) -> Result<Operation, u8> {
    return Err(1)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_ops() {
        let mut ops = Ops::new();
        ops.load_ops();

        assert_eq!(ops.ops.get(&0x3e).unwrap().mnemonic, "LD")
    }
}