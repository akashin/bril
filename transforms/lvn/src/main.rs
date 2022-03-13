use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    functions: Vec<Function>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Function {
    name: String,

    instrs: Vec<Instruction>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Instruction {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    op: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dest: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<serde_json::Value>,

    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    type_: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    labels: Vec<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

impl Instruction {
    fn is_terminator(&self) -> bool {
        match &self.op {
            Some(op) => {
                op == "jmp" || op == "cond"
            }
            None => false,
        }
    }
}

#[derive(Debug)]
struct Block {
    instrs: Vec<Instruction>,
    next_blocks: Vec<i64>,
}

#[derive(Debug)]
struct ControlFlowGraph {
    blocks: Vec<Block>,
}

impl ControlFlowGraph {
    fn to_instrs(&self) -> Vec<Instruction> {
        let mut result = Vec::<Instruction>::new();
        for block in &self.blocks {
            result.extend(block.instrs.clone());
        }
        result
    }
}

fn construct_control_flow_graph(function: &Function) -> ControlFlowGraph {
    let mut cfg = ControlFlowGraph { blocks: Vec::new() };

    let mut cur_block = Block {
        instrs: Vec::new(),
        next_blocks: Vec::new(),
    };

    for instr in &function.instrs {
        cur_block.instrs.push(instr.clone());
        if instr.is_terminator() {
            cfg.blocks.push(cur_block);
            cur_block = Block {
                instrs: Vec::new(),
                next_blocks: Vec::new(),
            };
        }
    }
    if !cur_block.instrs.is_empty() {
        cfg.blocks.push(cur_block);
    }

    cfg
}

fn eliminate_dead_code(cfg: ControlFlowGraph) -> ControlFlowGraph {
    cfg
}

fn main() {
    let mut buffer = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buffer)
        .expect("Failed to read input");

    let mut program: Program = serde_json::from_str(&buffer).expect("Failed to parse program IR");
    for function in &mut program.functions {
        let cfg = construct_control_flow_graph(function);
        let cfg = eliminate_dead_code(cfg);
        dbg!(&cfg);
        function.instrs = cfg.to_instrs();
    }

    println!(
        "{}",
        serde_json::to_string(&program).expect("Failed to serialize program")
    );
}
