use std::io::prelude::*;

extern crate anyhow;
extern crate xml;

use anyhow::{anyhow, bail, Result};
use xml::reader::{EventReader, XmlEvent};

#[cfg(test)]
mod tests;

#[derive(Debug, Eq, PartialEq)]
enum Action {
    RawString(String),
    Attribute(String),
    AttributeWithDefault { attr: String, default: String },
}

#[derive(Debug, PartialEq, Eq)]
enum Instruction {
    StartTag { tag: String, actions: Vec<Action> },
    EndTag { tag: String, actions: Vec<Action> },
}

impl Instruction {
    fn actions_mut(&mut self) -> &mut Vec<Action> {
        match self {
            Instruction::StartTag { tag: _, actions } => actions,
            Instruction::EndTag { tag: _, actions } => actions,
        }
    }
}

fn process(instructions: &[Instruction], input: impl Read, mut output: impl Write) -> Result<()> {
    let reader = EventReader::new(input);

    for wev in reader {
        match wev? {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace: _,
            } => {
                for instruction in instructions.iter() {
                    match instruction {
                        Instruction::StartTag { tag, actions } if tag == &name.local_name => {
                            for action in actions {
                                match action {
                                    Action::RawString(s) => {
                                        output.write_all(s.as_bytes())?;
                                    }
                                    Action::Attribute(attr) => {
                                        let value = attributes.iter().filter_map(|a| if &a.name.local_name == attr { Some(&a.value) } else { None }).next().ok_or_else(|| anyhow!("No attribute {} found for element {}. Attributes: {}", attr, tag, attributes.iter().map(|a| a.name.local_name.as_str()).collect::<Vec<_>>().join(",") ))?;
                                        output.write_all(value.as_bytes())?;
                                    }
                                    Action::AttributeWithDefault { attr, default } => {
                                        match attributes
                                            .iter()
                                            .filter_map(|a| {
                                                if &a.name.local_name == attr {
                                                    Some(&a.value)
                                                } else {
                                                    None
                                                }
                                            })
                                            .next()
                                        {
                                            Some(value) => output.write_all(value.as_bytes())?,
                                            None => output.write_all(default.as_bytes())?,
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            XmlEvent::EndElement { name } => {
                for instruction in instructions.iter() {
                    match instruction {
                        Instruction::EndTag { tag, actions } if tag == &name.local_name => {
                            for action in actions {
                                match action {
                                    Action::RawString(s) => {
                                        output.write_all(s.as_bytes())?;
                                    }
                                    _ => {
                                        todo!()
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn parse_to_instructions(argv: &[&str]) -> Result<Vec<Instruction>> {
    let mut instructions = vec![];
    let mut argv = argv.iter();
    let mut current_instruction: Option<Instruction> = None;
    while let Some(arg) = argv.next() {
        match *arg {
            "-s" => {
                let tag = argv.next().ok_or(anyhow!("Need an argument for -s"))?;
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::StartTag {
                    tag: tag.to_string(),
                    actions: vec![],
                });
            }
            "-e" => {
                let tag = argv.next().ok_or(anyhow!("Need an argument for -e"))?;
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::EndTag {
                    tag: tag.to_string(),
                    actions: vec![],
                });
            }

            "-o" => {
                let string = argv.next().ok_or(anyhow!("Need an argument for -o"))?;
                match current_instruction {
                    None => {
                        bail!("Cannot use -o before you have done a -s/-e");
                    }
                    Some(ref mut i) => {
                        i.actions_mut().push(Action::RawString(string.to_string()));
                    }
                }
            }
            "--nl" => match current_instruction {
                None => {
                    bail!("Cannot use --nl before you have done a -s/-e");
                }
                Some(ref mut i) => {
                    i.actions_mut().push(Action::RawString("\n".to_string()));
                }
            },

            "-v" => {
                let attr = argv.next().ok_or(anyhow!("Need an argument for -v"))?;
                match current_instruction {
                    None => {
                        bail!("Cannot use -v before you have done a -s/-e");
                    }
                    Some(ref mut i) => {
                        i.actions_mut().push(Action::Attribute(attr.to_string()));
                    }
                }
            }

            "-V" => {
                let attr = argv
                    .next()
                    .ok_or(anyhow!("Need an attribute argument for -V"))?
                    .to_string();
                let default = argv
                    .next()
                    .ok_or(anyhow!("Need a default argument for -V"))?
                    .to_string();
                match current_instruction {
                    None => {
                        bail!("Cannot use -V before you have done a -s/-e");
                    }
                    Some(ref mut i) => {
                        i.actions_mut()
                            .push(Action::AttributeWithDefault { attr, default });
                    }
                }
            }

            arg => {
                todo!("unknown arg: {}", arg);
            }
        }
    }

    if let Some(previous) = current_instruction.take() {
        instructions.push(previous);
    }

    Ok(instructions)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    if args.is_empty() || args == vec!["-h"] {
        println!("anglosaxon");
        return Ok(());
    }

    let mut stdin = std::io::stdin();
    let stdout = std::io::stdout();

    let instructions = parse_to_instructions(args.as_slice())?;

    process(&instructions, &mut stdin, stdout)?;

    Ok(())
}
