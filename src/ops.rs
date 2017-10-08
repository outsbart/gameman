use std::collections::HashMap;


pub struct Operation {
    code: u16,
    bytes: u8,              // How many bytes is it long?
    mnemonic: String,
    flags: [char; 4],       // Z H N C
    operands: [char; 2],    // second operand is optional 'x'?
    cycles: [u8; 2],
}

impl Operation {

}

pub struct Ops {
    operations: HashMap<u8, Operation>
}

impl Ops {
    pub fn new() -> Ops { Ops { operations: HashMap::new() } }
    pub fn load_ops() {

    }
}

/// returns an error with the amount of extra bytes required to decode the operation
pub fn decode_op(op: u16) -> Result<Operation, u8> {
    return Err(1)
}

