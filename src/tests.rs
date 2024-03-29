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
            actions: vec![Action::Attribute("id".to_string(), Filters::default()),]
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
            actions: vec![Action::AttributeWithDefault(
                "id".to_string(),
                "NOID".to_string(),
                Filters::default()
            ),]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "1\nNOID\n"
);

assert_flow!(
    attribute_with_parent_value1,
    r#"<notes><note id="1">hello<comment id="10">foo</comment><comment id="11">bar</comment></note><note>hi</note></notes>"#,
    vec![
        Instruction::StartTag {
            tag: "comment".to_string(),
            actions: vec![
                Action::Attribute("id".to_string(), Filters::default()),
                Action::RawString(".".to_string()),
                Action::ParentAttribute(1, "id".to_string(), Filters::default()),
            ]
        },
        Instruction::EndTag {
            tag: "comment".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "10.1\n11.1\n"
);

assert_flow!(
    start_doc,
    r#"<notes><note id="1">hello<comment id="10">foo</comment><comment id="11">bar</comment></note><note>hi</note></notes>"#,
    vec![
        Instruction::StartDocument {
            actions: vec![Action::RawString("startdoc".to_string()),]
        },
        Instruction::StartTag {
            tag: "notes".to_string(),
            actions: vec![Action::RawString(".notes.".to_string()),]
        },
        Instruction::EndDocument {
            actions: vec![Action::RawString("enddoc".to_string()),]
        },
    ],
    "startdoc.notes.enddoc"
);

assert_flow!(
    attribute_with_parent_value2,
    r#"<notes><note id="1">hello<comment id="10">foo</comment><comment id="11">bar</comment></note><note>hi<comment id="20">foo</comment></note></notes>"#,
    vec![
        Instruction::StartTag {
            tag: "comment".to_string(),
            actions: vec![
                Action::Attribute("id".to_string(), Filters::default()),
                Action::RawString(".".to_string()),
                Action::ParentAttributeWithDefault(
                    1,
                    "id".to_string(),
                    "NOID".to_string(),
                    Filters::default()
                ),
            ]
        },
        Instruction::EndTag {
            tag: "comment".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "10.1\n11.1\n20.NOID\n"
);

assert_flow!(
    attribute_with_filter1,
    r#"<notes><note id="1">hello</note><note id="2">hi</note></notes>"#,
    vec![
        Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute("id".to_string(), Filters::default()),]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "1\n2\n"
);

assert_flow!(
    attribute_with_filter2,
    "<notes><note author=\"foo\nbar\">hello</note><note author=\"ok\">hi</note></notes>",
    vec![
        Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute(
                "author".to_string(),
                Filters(vec![TextFilter::TSVEscape])
            ),]
        },
        Instruction::EndTag {
            tag: "note".to_string(),
            actions: vec![Action::RawString("\n".to_string()),]
        },
    ],
    "foo\\nbar\nok\n"
);

mod parse {
    use super::*;

    macro_rules! assert_parse {
        ($name:ident, $input:expr, $expected_output:expr) => {
            #[test]
            fn $name() {
                let input = $input;
                let input: Vec<_> = input.split(" ").collect();
                let (_config, actual_output) = parse_to_instructions(input.as_slice()).unwrap();

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
        simple_note4,
        "-s note -o notestart --tab",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![
                Action::RawString("notestart".to_string()),
                Action::RawString("\t".to_string()),
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
            actions: vec![Action::Attribute("id".to_string(), Filters::default()),]
        },]
    );

    assert_parse!(
        value2,
        "-s note -v ./id",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute("id".to_string(), Filters::default()),]
        },]
    );

    assert_parse!(
        value_filter1,
        "-s note -v ./id!tsv",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::Attribute(
                "id".to_string(),
                Filters(vec![TextFilter::TSVEscape])
            ),]
        },]
    );

    assert_parse!(
        value_with_two_tabs,
        "-s note -v id --tab -v class --tab -v uid --nl",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![
                Action::Attribute("id".to_string(), Filters::default()),
                Action::RawString("\t".to_string()),
                Action::Attribute("class".to_string(), Filters::default()),
                Action::RawString("\t".to_string()),
                Action::Attribute("uid".to_string(), Filters::default()),
                Action::RawString("\n".to_string()),
            ]
        },]
    );

    assert_parse!(
        value_with_default1,
        "-s note -V id NOID",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::AttributeWithDefault(
                "id".to_string(),
                "NOID".to_string(),
                Filters::default()
            ),]
        },]
    );

    assert_parse!(
        value_with_default2,
        "-s note -V ./id NOID",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::AttributeWithDefault(
                "id".to_string(),
                "NOID".to_string(),
                Filters::default()
            ),]
        },]
    );

    assert_parse!(
        value_with_default_two_tabs,
        "-s note -V id NOID --tab -V class NOCLASS --tab -V uid NOUID --nl",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![
                Action::AttributeWithDefault(
                    "id".to_string(),
                    "NOID".to_string(),
                    Filters::default()
                ),
                Action::RawString("\t".to_string()),
                Action::AttributeWithDefault(
                    "class".to_string(),
                    "NOCLASS".to_string(),
                    Filters::default()
                ),
                Action::RawString("\t".to_string()),
                Action::AttributeWithDefault(
                    "uid".to_string(),
                    "NOUID".to_string(),
                    Filters::default()
                ),
                Action::RawString("\n".to_string()),
            ]
        },]
    );

    assert_parse!(
        parent_attr1,
        "-s note -v ../id",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::ParentAttribute(
                1,
                "id".to_string(),
                Filters::default()
            ),],
        },]
    );

    assert_parse!(
        parent_attr2,
        "-s note -v ../../id",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::ParentAttribute(
                2,
                "id".to_string(),
                Filters::default()
            ),],
        },]
    );

    assert_parse!(
        parent_attr_with_default1,
        "-s note -V ../../id NOID",
        vec![Instruction::StartTag {
            tag: "note".to_string(),
            actions: vec![Action::ParentAttributeWithDefault(
                2,
                "id".to_string(),
                "NOID".to_string(),
                Filters::default()
            ),],
        },]
    );

    assert_parse!(
        start_doc,
        "-S -o foo",
        vec![Instruction::StartDocument {
            actions: vec![Action::RawString("foo".to_string())]
        },]
    );
}

mod filters {
    use super::*;
    #[test]
    fn parse1() {
        assert_eq!(
            Filters::parse_both("id!none").unwrap(),
            ("id".to_string(), Filters(vec![TextFilter::Nothing]))
        );
        assert_eq!(
            Filters::parse_both("id!none!none").unwrap(),
            (
                "id".to_string(),
                Filters(vec![TextFilter::Nothing, TextFilter::Nothing])
            )
        );
    }

    macro_rules! assert_filter {
        ($name:ident, $filters:expr, $input:expr, $expected_output:expr) => {
            #[test]
            fn $name() {
                let input = $input;
                let expected_output = $expected_output;
                let ff = Filters::parse_both($filters).unwrap();
                let filters = ff.1;
                assert_eq!(filters.apply(input), $expected_output);
            }
        };
    }

    assert_filter!(empty1, "x", "foo", "foo");
    assert_filter!(empty2, "x!none", "foo", "foo");

    assert_filter!(unix1, "x!unix", "foo", "foo");
    assert_filter!(unix2, "x!unix", "foo bar", "foo bar");
    assert_filter!(unix3, "x!unix", "foo\tbar", "foo\\tbar");
    assert_filter!(unix4, "x!unix", "foo\rbar", "foo\\rbar");

    assert_filter!(tsv1, "x!tsv", "foo bar", "foo bar");
    assert_filter!(tsv2, "x!tsv", "foo\rbar", "foo\\rbar");
    assert_filter!(tsv3, "x!tsv", "foo\tbar", "foo\\tbar");
    assert_filter!(tsv4, "x!tsv", "foo\" \"bar", "foo\" \"bar");
}
