use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
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

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
enum Expression {
    Op(String, Vec<usize>),
    Const(i64),
}

fn run_local_value_numbering(block: &mut Block) -> bool {
    let mut variable_to_number: HashMap<String, usize> = HashMap::new();
    let mut expression_to_number: HashMap<Expression, usize> = HashMap::new();
    let mut number_to_expression: HashMap<usize, Expression> = HashMap::new();
    let mut next_number = 0;
    let mut used_numbers = HashSet::new();
    let mut instruction_numbers = Vec::new();
    for instr in &block.instrs {
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
            let number = *expression_to_number
                .entry(expression.clone())
                .or_insert_with(|| {
                    next_number += 1;
                    next_number - 1
                });
            number_to_expression.insert(number, expression);
            // Update the mapping from variable name (dest) to value number.
            variable_to_number.insert(dest.clone(), number);
            instruction_numbers.push(Some(number));
        } else {
            if instr.op.is_some() {
                for arg in &instr.args {
                    used_numbers
                        .insert(*variable_to_number.get(arg).expect("No number for variable"));
                }
            }
            instruction_numbers.push(None);
        }
    }
    let mut queue = VecDeque::new();
    queue.extend(used_numbers.clone());

    while !queue.is_empty() {
        let number = queue.pop_front().unwrap();
        let expression = number_to_expression.get(&number).unwrap();
        match expression {
            Expression::Op(_, args) => {
                for arg in args {
                    if used_numbers.contains(arg) {
                        continue;
                    }
                    used_numbers.insert(*arg);
                    queue.push_back(*arg);
                }
            }
            Expression::Const(_) => {
                // We just mark this instruction as used.
            }
        }
    }

    // Remove unused instructions.
    let mut new_instrs = Vec::new();
    let mut number_to_canonical_dest: HashMap<usize, String> = HashMap::new();
    let mut new_variable_to_number: HashMap<String, usize> = HashMap::new();
    for (i, instr) in block.instrs.iter().enumerate() {
        if let Some(number) = instruction_numbers[i] {
            new_variable_to_number.insert(instr.dest.clone().unwrap(), number);
            if used_numbers.contains(&number) {
                number_to_canonical_dest.insert(number, instr.dest.clone().unwrap());

                let mut new_instr = instr.clone();
                for arg in new_instr.args.iter_mut() {
                    let arg_number = new_variable_to_number
                        .get(arg)
                        .expect("No number for variable");
                    *arg = number_to_canonical_dest
                        .get(arg_number)
                        .expect("No canonical dest for number")
                        .clone();
                }
                new_instrs.push(new_instr);
                used_numbers.remove(&number);
            }
        } else {
            let mut new_instr = instr.clone();
            for arg in new_instr.args.iter_mut() {
                let arg_number = new_variable_to_number
                    .get(arg)
                    .expect("No number for variable");
                *arg = number_to_canonical_dest
                    .get(arg_number)
                    .expect("No canonical dest for number")
                    .clone();
            }
            new_instrs.push(new_instr);
        }
    }
    block.instrs = new_instrs;

    false
}

fn eliminate_dead_code(mut cfg: ControlFlowGraph) -> ControlFlowGraph {
    for block in cfg.blocks.iter_mut() {
        run_local_value_numbering(block);
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
