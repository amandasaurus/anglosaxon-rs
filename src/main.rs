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

    ParentAttribute(usize, String),
    ParentAttributeWithDefault(usize, String, String),
}

impl Action {
    fn is_parent_attr(&self) -> bool {
        match self {
            Action::ParentAttribute(_, _) | Action::ParentAttributeWithDefault(_, _, _) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Instruction {
    StartDocument { actions: Vec<Action> },
    StartTag { tag: String, actions: Vec<Action> },
    EndTag { tag: String, actions: Vec<Action> },
    EndDocument { actions: Vec<Action> },
}

impl Instruction {
    fn actions(&self) -> &[Action] {
        match self {
            Instruction::StartDocument { actions } => actions,
            Instruction::StartTag { tag: _, actions } => actions,
            Instruction::EndTag { tag: _, actions } => actions,
            Instruction::EndDocument { actions } => actions,
        }
    }
    fn actions_mut(&mut self) -> &mut Vec<Action> {
        match self {
            Instruction::StartDocument { actions } => actions,
            Instruction::StartTag { tag: _, actions } => actions,
            Instruction::EndTag { tag: _, actions } => actions,
            Instruction::EndDocument { actions } => actions,
        }
    }
}

fn get_attr<'a>(
    attributes: &'a [xml::attribute::OwnedAttribute],
    attr: &str,
    tag: &str,
) -> Result<&'a str> {
    attributes
        .iter()
        .filter_map(|a| {
            if &a.name.local_name == attr {
                Some(a.value.as_str())
            } else {
                None
            }
        })
        .next()
        .ok_or_else(|| {
            anyhow!(
                "No attribute {} found for element {}. Attributes: {}",
                attr,
                tag,
                attributes
                    .iter()
                    .map(|a| a.name.local_name.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
}

fn process(instructions: &[Instruction], input: impl Read, mut output: impl Write) -> Result<()> {
    let reader = EventReader::new(input);

    let has_parent_attributes = instructions
        .iter()
        .any(|i| i.actions().iter().any(|a| a.is_parent_attr()));
    let mut parent_attrs: Vec<Vec<xml::attribute::OwnedAttribute>> = vec![];
    let mut parent_tags: Vec<String> = vec![];

    for wev in reader {
        match wev? {
            XmlEvent::StartDocument {
                version: _,
                encoding: _,
                standalone: _,
            } => {
                for instruction in instructions.iter() {
                    match instruction {
                        Instruction::StartDocument { actions } => {
                            for action in actions {
                                match action {
                                    Action::RawString(s) => {
                                        output.write_all(s.as_bytes())?;
                                    }
                                    _ => todo!(),
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

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
                                        let value = get_attr(&attributes, attr, tag)?;
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
                                    Action::ParentAttribute(level, attr) => {
                                        if *level > parent_attrs.len() {
                                            bail!("")
                                        }
                                        let value = get_attr(
                                            &parent_attrs[parent_attrs.len() - level],
                                            attr,
                                            parent_tags[parent_attrs.len() - level].as_str(),
                                        )?;
                                        output.write_all(value.as_bytes())?;
                                    }
                                    _ => todo!(),
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if has_parent_attributes {
                    parent_attrs.push(attributes);
                    parent_tags.push(name.local_name);
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
                if has_parent_attributes {
                    parent_attrs.pop();
                    parent_tags.pop();
                }
            }

            XmlEvent::EndDocument => {
                for instruction in instructions.iter() {
                    match instruction {
                        Instruction::EndDocument { actions } => {
                            for action in actions {
                                match action {
                                    Action::RawString(s) => {
                                        output.write_all(s.as_bytes())?;
                                    }
                                    _ => todo!(),
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
    let mut level;
    while let Some(arg) = argv.next() {
        match *arg {
            "-S" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::StartDocument { actions: vec![] });
            }
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
            "-E" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::EndDocument { actions: vec![] });
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
                let mut attr: &str = argv.next().ok_or(anyhow!("Need an argument for -v"))?;
                match current_instruction {
                    None => {
                        bail!("Cannot use -v before you have done a -s/-e");
                    }
                    Some(ref mut i) => {
                        level = 0;
                        while attr.starts_with("../") {
                            level += 1;
                            attr = attr.strip_prefix("../").unwrap();
                        }
                        if level == 0 {
                            i.actions_mut().push(Action::Attribute(attr.to_string()));
                        } else {
                            i.actions_mut()
                                .push(Action::ParentAttribute(level, attr.to_string()));
                        }
                    }
                }
            }

            "-V" => {
                let mut attr: &str = argv
                    .next()
                    .ok_or(anyhow!("Need an attribute argument for -V"))?;
                let default = argv
                    .next()
                    .ok_or(anyhow!("Need a default argument for -V"))?
                    .to_string();
                match current_instruction {
                    None => {
                        bail!("Cannot use -V before you have done a -s/-e");
                    }
                    Some(ref mut i) => {
                        level = 0;
                        while attr.starts_with("../") {
                            level += 1;
                            attr = attr.strip_prefix("../").unwrap();
                        }
                        if level == 0 {
                            i.actions_mut().push(Action::AttributeWithDefault {
                                attr: attr.to_string(),
                                default,
                            });
                        } else {
                            i.actions_mut().push(Action::ParentAttributeWithDefault(
                                level,
                                attr.to_string(),
                                default,
                            ));
                        }
                    }
                }
            }

            arg => {
                bail!("unknown arg: {}", arg)
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
