use std::io::prelude::*;

extern crate anyhow;
extern crate clap;
extern crate xml;

use anyhow::{anyhow, bail, Result};
use clap::{App, Arg};
use xml::reader::{EventReader, XmlEvent};

#[cfg(test)]
mod tests;

#[derive(Debug, Eq, PartialEq)]
enum Action {
    RawString(String),
    Attribute(String),
    AttributeWithDefault(String, String),

    ParentAttribute(usize, String),
    ParentAttributeWithDefault(usize, String, String),
}

impl Action {
    fn is_parent_attr(&self) -> bool {
        matches!(
            self,
            Action::ParentAttribute(_, _) | Action::ParentAttributeWithDefault(_, _, _)
        )
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
            if a.name.local_name == attr {
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

/// The main "inner main"
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
                    if let Instruction::StartDocument { actions } = instruction {
                        for action in actions {
                            match action {
                                Action::RawString(s) => {
                                    output.write_all(s.as_bytes())?;
                                }
                                _ => todo!(),
                            }
                        }
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
                                    Action::AttributeWithDefault(attr, default) => match attributes
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
                                    },

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
                                    Action::ParentAttributeWithDefault(level, attr, default) => {
                                        if *level > parent_attrs.len() {
                                            bail!("")
                                        }
                                        match parent_attrs[parent_attrs.len() - level]
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
                    if let Instruction::EndDocument { actions } = instruction {
                        for action in actions {
                            match action {
                                Action::RawString(s) => {
                                    output.write_all(s.as_bytes())?;
                                }
                                _ => todo!(),
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    Ok(())
}

/// Parses this args (could be argv) to the instructions
fn parse_to_instructions<'a>(argv: impl Into<Option<&'a [&'a str]>>) -> Result<Vec<Instruction>> {
    let mut instructions = vec![];
    let app = clap_app();
    let argv: Option<&[&str]> = argv.into();
    let args = clap_app_to_ordered_matches(app, argv);

    let mut current_instruction: Option<Instruction> = None;
    let mut level: usize;
    for (name, mut value) in args.into_iter() {
        match name.as_str() {
            "startdoc" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::StartDocument { actions: vec![] });
            }
            "startelement" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::StartTag {
                    tag: value.remove(0),
                    actions: vec![],
                });
            }
            "endelement" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                let tag = value.remove(0);
                current_instruction = Some(Instruction::EndTag {
                    tag,
                    actions: vec![],
                });
            }
            "enddoc" => {
                if let Some(previous) = current_instruction.take() {
                    instructions.push(previous);
                }
                current_instruction = Some(Instruction::EndDocument { actions: vec![] });
            }

            "raw" => match current_instruction {
                None => {
                    bail!("Cannot use -o before you have done a -s/-e");
                }
                Some(ref mut i) => {
                    i.actions_mut().push(Action::RawString(value.remove(0)));
                }
            },
            "newline" => match current_instruction {
                None => {
                    bail!("Cannot use --nl before you have done a -s/-e");
                }
                Some(ref mut i) => {
                    i.actions_mut().push(Action::RawString("\n".to_string()));
                }
            },
            "tab" => match current_instruction {
                None => {
                    bail!("Cannot use --tab before you have done a -s/-e");
                }
                Some(ref mut i) => {
                    i.actions_mut().push(Action::RawString("\t".to_string()));
                }
            },

            "value" => {
                // TODO is it possible do .strip_prefix (equiv.) on String, not just str
                let attr = value.remove(0);
                let mut attr = attr.as_str();
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

            "value_with_default" => match current_instruction {
                None => {
                    bail!("Cannot use -V before you have done a -s/-e");
                }
                Some(ref mut i) => {
                    let attr = value.remove(0);
                    let mut attr = attr.as_str();
                    let default = value.remove(0);
                    level = 0;
                    while attr.starts_with("../") {
                        level += 1;
                        attr = attr.strip_prefix("../").unwrap();
                    }
                    if level == 0 {
                        i.actions_mut()
                            .push(Action::AttributeWithDefault(attr.to_string(), default));
                    } else {
                        i.actions_mut().push(Action::ParentAttributeWithDefault(
                            level,
                            attr.to_string(),
                            default,
                        ));
                    }
                }
            },

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

fn clap_app_to_ordered_matches(
    app: clap::App,
    argv: Option<&[&str]>,
) -> Vec<(String, Vec<String>)> {
    let args: Vec<(&str, usize)> = app
        .get_arguments()
        .map(|a| {
            (
                a.get_name(),
                a.get_num_vals().unwrap_or_else(|| {
                    if a.is_set(clap::ArgSettings::TakesValue) {
                        1
                    } else {
                        0
                    }
                }),
            )
        })
        .filter(|&(a, _)| a != "version")
        .collect::<Vec<_>>();

    let matches = match argv {
        // from CLI args
        None => app.get_matches(),

        // From the provided args (used for testing)
        Some(argv) => {
            let app = app.setting(clap::AppSettings::NoBinaryName);
            app.get_matches_from(argv)
        }
    };

    let mut results = vec![];
    for (name, num_vals) in args {
        if matches.occurrences_of(name) == 0 {
            // argument not used
            continue;
        }
        let indices = matches.indices_of(name).unwrap();

        if num_vals == 0 {
            results.extend(indices.map(|i| (i, (name.to_string(), vec![]))));
        } else {
            let indices = indices.collect::<Vec<_>>();
            let indices = indices.chunks(num_vals).collect::<Vec<_>>();
            let values = matches
                .values_of(name)
                .unwrap()
                .map(|v| v.to_string())
                .collect::<Vec<_>>();
            let values = values.chunks(num_vals).collect::<Vec<_>>();
            results.extend(
                indices
                    .iter()
                    .zip(values)
                    .map(|(i, v)| (i[0], (name.to_string(), v.to_vec()))),
            );
        }
    }

    results.sort_by_key(|x| x.0);

    results
        .into_iter()
        .map(|(_i, (name, vals))| (name, vals))
        .collect()
}

/// Creates our clap app
fn clap_app() -> clap::App<'static> {
    App::new("anglosaxon")
        .arg(
            Arg::new("startdoc")
                .short('S').long("startdoc")
                .help("Event happens once, at the start of the XML document")
                .takes_value(false)
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("startelement")
                .short('s').long("start")
                .help("Event happens when this tag is opened")
                .takes_value(true).value_name("TAG")
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("endelement")
                .short('e').long("end")
                .help("Event happens when this tag is closed")
                .takes_value(true).value_name("TAG")
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("enddoc")
                .short('E').long("enddoc")
                .help("Event happens once, at the end of the XML document")
                .takes_value(false)
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("raw")
                .short('o').long("output")
                .help("Outputs this string")
                .takes_value(true).value_name("STRING")
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("value")
                .short('v').long("value")
                .help("Outputs the value of this XML attribute, an error occurs if that attribute isn't present")
                .value_name("ATTRIBUTE")
                .takes_value(true)
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("value_with_default")
                .short('V').long("value-default")
                .help("Outputs this string")
                .takes_value(true)
                .value_name("ATTRIBUTE DEFAULT")
                .number_of_values(2)
                .multiple_occurrences(true)
                .use_delimiter(false),
        )
        .arg(
            Arg::new("newline")
                .long("nl")
                .help("Outputs a new line character")
                .takes_value(false)
                .multiple_occurrences(true),
        )
        .arg(
            Arg::new("tab")
                .long("tab")
                .help("Outputs a tab character")
                .takes_value(false)
                .multiple_occurrences(true),
        )
}

fn main() -> Result<()> {
    let mut stdin = std::io::stdin();
    let stdout = std::io::stdout();

    let instructions = parse_to_instructions(None)?;
    if instructions.is_empty() {
        clap_app().print_long_help()?;
        return Ok(());
    }

    process(&instructions, &mut stdin, stdout)?;

    Ok(())
}
