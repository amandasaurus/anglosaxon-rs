use super::*;
use std::io::Cursor;

macro_rules! assert_flow {
    ($name:ident, $input:expr, $instructions:expr, $expected_output:expr) => {
        #[test]
        fn $name() {
            let input = $input;
            let expected_output = $expected_output;
            let mut output: Vec<u8> = vec![];

            //let instructions = vec![
            //    Instruction::StartTag{ tag: "note".to_string(), actions: vec![
            //        Action::RawString("notestart".to_string()),
            //    ] },
            //];
            let instructions = $instructions;

            process(&instructions, input.as_bytes(), Cursor::new(&mut output)).unwrap();

            assert_eq!(String::from_utf8(output).unwrap(), expected_output);
        }
    };
}

assert_flow!(
    simple1,
    "<note>hello</note>",
    vec![Instruction::StartTag {
        tag: "note".to_string(),
        actions: vec![Action::RawString("notestart".to_string()),]
    },],
    "notestart"
);

assert_flow!(
    simple2,
    "<note>hello</note><note>hi</note>",
    vec![Instruction::StartTag {
        tag: "note".to_string(),
        actions: vec![Action::RawString("notestart".to_string()),]
    },],
    "notestartnotestart"
);

assert_flow!(
    simple3,
    "<note>hello<note>hi</note></note>",
    vec![Instruction::StartTag {
        tag: "note".to_string(),
        actions: vec![Action::RawString("notestart".to_string()),]
    },],
    "notestartnotestart"
);

assert_flow!(
    simple_end_1,
    "<note>hello<note>hi</note></note>",
    vec![
        Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("notestart ".to_string()),]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("noteend ".to_string()),]
        },
    ],
    "notestart notestart noteend noteend "
);

assert_flow!(
    attribute1,
    r#"<notes><note id="1">hello</note><note id="2">hi</note></notes>"#,
    vec![
        Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute("id".to_string()),]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "1\n2\n"
);

assert_flow!(
    attribute_with_default1,
    r#"<notes><note id="1">hello</note><note>hi</note></notes>"#,
    vec![
        Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::AttributeWithDefault {
                attr: "id".to_string(),
                default: "NOID".to_string()
            },]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "1\nNOID\n"
);

mod parse {
    use super::*;

    macro_rules! assert_parse {
        ($name:ident, $input:expr, $expected_output:expr) => {
            #[test]
            fn $name() {
                let input = $input;
                let input: Vec<_> = input.split(" ").collect();
                let actual_output = parse_to_instructions(&input).unwrap();

                assert_eq!(actual_output, $expected_output);
            }
        };
    }

    assert_parse!(
        simple_note1,
        "-s note -o notestart",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("notestart".to_string())]
        }]
    );

    assert_parse!(
        simple_note2,
        "-s note -o notestart -o foo",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![
                Action::RawString("notestart".to_string()),
                Action::RawString("foo".to_string()),
            ]
        }]
    );

    assert_parse!(
        simple_note3,
        "-s note -o notestart --nl",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![
                Action::RawString("notestart".to_string()),
                Action::RawString("\n".to_string()),
            ]
        }]
    );

    assert_parse!(
        start_end_1,
        "-s note -o notestart -e note -o foo",
        vec![
            Instruction::StartTag {
                tag: "note".to_string(),
                actions: vec![Action::RawString("notestart".to_string()),]
            },
            Instruction::EndTag {
                tag: "note".to_string(),
                actions: vec![Action::RawString("foo".to_string()),]
            },
        ]
    );

    assert_parse!(
        value1,
        "-s note -v id",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute("id".to_string()),]
        },]
    );

    assert_parse!(
        value_with_default1,
        "-s note -V id NOID",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::AttributeWithDefault {
                attr: "id".to_string(),
                default: "NOID".to_string()
            },]
        },]
    );
}
