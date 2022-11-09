use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Program {
    functions: Vec<Function>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NamedArg {
    name: String,

    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Function {
    name: String,

    instrs: Vec<Instruction>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<NamedArg>,
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
            Some(op) => op == "jmp" || op == "br" || op == "ret",
            None => false,
        }
    }

    fn is_label(&self) -> bool {
        self.label.is_some()
    }
}

#[derive(Debug, Default)]
struct Block {
    instrs: Vec<Instruction>,
    next_blocks: Vec<usize>,
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

    let mut cur_block = Block::default();
    let mut flush_block = |block: &mut Block| {
        if !block.instrs.is_empty() {
            cfg.blocks.push(std::mem::take(block));
        }
    };

    for instr in &function.instrs {
        // Label is always starting a new block.
        if instr.is_label() {
            flush_block(&mut cur_block);
        }

        cur_block.instrs.push(instr.clone());

        // Terminator always ends the block.
        if instr.is_terminator() {
            flush_block(&mut cur_block);
        }
    }
    flush_block(&mut cur_block);

    // Populate mapping from labels to block indices.
    let mut label_to_block_index: HashMap<String, usize> = HashMap::new();
    for (i, block) in cfg.blocks.iter().enumerate() {
        if let Some(label) = &block.instrs[0].label {
            label_to_block_index.insert(label.clone(), i);
        }
    }

    // Populate next block pointers.
    for i in 0..cfg.blocks.len() {
        let block = &mut cfg.blocks[i];
        if let Some(instr) = block.instrs.last() {
            if instr.is_terminator() {
                for label in &instr.labels {
                    block
                        .next_blocks
                        .push(*label_to_block_index.get(label).expect("Label not found"));
                }
            } else {
                block.next_blocks.push(i + 1);
            }
        }
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
        for (i, block) in cfg.blocks.iter().enumerate() {
            eprintln!("{i} -> {:?}", block.next_blocks);
        }
        function.instrs = cfg.to_instrs();
    }

    println!(
        "{}",
        serde_json::to_string(&program).expect("Failed to serialize program")
    );
}
