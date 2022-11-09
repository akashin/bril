use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
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
    // value: Option<serde_json::Value>,
    value: Option<i64>,

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

#[derive(PartialEq, Eq, Hash, Debug)]
enum Expression {
    Op(String, Vec<usize>),
    Const(i64),
}

fn remove_unused_instructions(block: &mut Block) -> bool {
    let mut variable_to_number: HashMap<String, usize> = HashMap::new();
    let mut expression_to_number: HashMap<Expression, usize> = HashMap::new();
    let mut next_number = 0;
    let mut used_variables = HashSet::new();
    for instr in &block.instrs {
        // dbg!(&instr);
        if let Some(dest) = &instr.dest {
            let op = instr.op.as_ref().expect("No op found").clone();
            let expression = if op == "const" {
                Expression::Const(instr.value.unwrap())
            } else {
                // Convert args to value numbers.
                let args: Vec<usize> = instr
                    .args
                    .iter()
                    .map(|arg| *variable_to_number.get(arg).expect("No number for variable"))
                    .collect();
                // Construct expression (op, vn1, vn2, ...)
                Expression::Op(op, args)
            };
            // Look it up, create if missing or reuse.
            let number = *expression_to_number.entry(expression).or_insert_with(|| {
                next_number += 1;
                next_number - 1
            });
            // Update the mapping from variable name (dest) to value number.
            variable_to_number.insert(dest.clone(), number);
        } else {
            if let Some(_) = &instr.op {
                for arg in &instr.args {
                    used_variables.insert(arg);
                }
            }
        }
    }
    dbg!(variable_to_number);
    dbg!(expression_to_number);
    false
}

fn eliminate_dead_code(mut cfg: ControlFlowGraph) -> ControlFlowGraph {
    for block in cfg.blocks.iter_mut() {
        remove_unused_instructions(block);
    }
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
        function.instrs = cfg.to_instrs();
    }

    println!(
        "{}",
        serde_json::to_string(&program).expect("Failed to serialize program")
    );
}
