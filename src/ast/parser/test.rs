use rstest::rstest;

use crate::ast::{AstEntry, AstParser, AstTree};

#[rstest]
#[case(b"")]
#[case(b" ")]
#[case(b"\t")]
#[case(b"\n")]
#[case(b"#")]
#[case(b"#\n")]
#[case(b"#\r\n")]
#[case(b"\r\n")]
#[case(b" \t\n\r\n")]
#[case(b"\n# this is a comment")]
#[case(b"\n# this is a comment\n#")]
#[case(b"\n# this is a comment\n#\n")]
fn parse_empty_ast(#[case] input: &[u8]) {
    let ast = AstParser::new()
        .parse_bytes(input.to_vec())
        .parse_into_tree();
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), AstTree::new());
}

#[rstest]
#[case(
        b"KEY=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY_WITH_UNDERSCORES=UNQUOTED_STRING/0123456789.",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY_WITH_UNDERSCORES".to_vec(), b"UNQUOTED_STRING/0123456789.".to_vec())
        ])
    )]
#[case(
        b"KEY=\"QUOTED String 0123456789 \\\\ \\\"\"",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String 0123456789 \\ \"".to_vec())
        ])
    )]
#[case(
        b"KEY=UNQUOTED_STRING;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY = UNQUOTED_STRING ;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY\n=\n\t    UNQUOTED_STRING;",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY=UNQUOTED_STRING;KEY2=\"QUOTED String @\";",
        AstTree::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY2".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"# comment before a key-value
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING
        # comment between key-values
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        # comment after key-value
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING # comment on same line as key-value
        KEY=\"QUOTED String @\";# comment on same line as key-value
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING# comment on same line as key-value
        KEY=\"QUOTED String @\"; # comment on same line as key-value
        ",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY+=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY-=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_remove(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY:=UNQUOTED_STRING",
        AstTree::from_iter(vec![
            AstEntry::new_assign_if_undefined(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY!",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY!
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY!NEXT-=VALUE",
        AstTree::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
        ])
    )]
#[case(
        b"KEY!!",
        AstTree::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !!",
        AstTree::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !!",
        AstTree::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY!!
        KEY=\"QUOTED String @\";
        ",
        AstTree::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY!!NEXT-=VALUE",
        AstTree::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec()),
            AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
        ])
    )]
fn parse_key_op_value(#[case] input: &[u8], #[case] output: AstTree) {
    use crate::ast::parser::AstParser;

    let ast = AstParser::new()
        .parse_bytes(input.to_vec())
        .parse_into_tree();
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), output);
}

#[rstest]
#[case(
        b"KEY: {}",
        AstTree::from_iter(vec![
            AstEntry::new_group(b"KEY".to_vec(), vec![])
        ])
    )]
#[case(
        b"      \t_   \n: {\t     }",
        AstTree::from_iter(vec![
            AstEntry::new_group(b"_".to_vec(), vec![])
        ])
    )]
#[case(
        b"KEY: {
            PART1 = VALUE;
            PART2 = other
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
#[case(
        b"# comment ahead of a group
        KEY: {
            PART1 = VALUE;
            PART2 = other
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
#[case(
        b"KEY: {
            # comment at the start of a group
            PART1 = VALUE;
            PART2 = other
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
#[case(
        b"KEY: {
            PART1 = VALUE;
            # comment in the middle of a group
            PART2 = other
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
#[case(
        b"KEY: {
            PART1 = VALUE;
            PART2 = other
            # comment at the end of a group
        }",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
#[case(
        b"KEY: {
            PART1 = VALUE;
            PART2 = other
        }# comment after a group",
        AstTree::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
fn parse_group(#[case] input: &[u8], #[case] output: AstTree) {
    let ast = AstParser::new()
        .parse_bytes(input.to_vec())
        .parse_into_tree();
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), output);
}
