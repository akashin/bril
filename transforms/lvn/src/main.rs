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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Instruction {
    op: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    dest: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<i64>,

    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    type_: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
}

fn main() {
    let mut buffer = String::new();
    std::io::stdin()
        .lock()
        .read_to_string(&mut buffer)
        .expect("Failed to read input");

    let program: Program = serde_json::from_str(&buffer).expect("Failed to parse program IR");

    println!(
        "{}",
        serde_json::to_string(&program).expect("Failed to serialize program")
    );
}
