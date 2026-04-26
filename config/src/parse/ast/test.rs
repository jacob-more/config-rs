use std::{ffi::OsStr, io::Read, os::unix::ffi::OsStrExt};

use rstest::rstest;

use crate::parse::{
    Ast, AstEntry, AstParser, BYTES_OPERATOR_ADD, BYTES_OPERATOR_ASSIGN,
    BYTES_OPERATOR_ASSIGN_IF_UNDEFINED, BYTES_OPERATOR_CLEAR, BYTES_OPERATOR_REMOVE,
    BYTES_OPERATOR_RESET,
};

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
    let ast = AstParser::new().parse(input.to_vec());
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), Ast::default());
}

#[rstest]
fn parse_key_assign_value(
    #[values(
        (b"A".as_slice(), b"A".as_slice()),
        (b"KEY".as_slice(), b"KEY".as_slice()),
        (b"\"KEY\"".as_slice(), b"KEY".as_slice()),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice()
        ),
        (
            b"\"Key with \\!\\@\\#\\$\\%\\^\\&\\*\\(\\)\\-\\=\\{\\}\\[\\]\\|\\\\\\:\\;\\\"\\'\\<\\>\\,\\.\\?\\/\\~\\` \\\"escape\\\" characters\"".as_slice(),
            b"Key with !@#$%^&*()-={}[]|\\:;\"'<>,.?/~` \"escape\" characters".as_slice()
        ),
    )]
    (raw_key, ast_key): (&'static [u8], &'static [u8]),
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    pre_op_whitespace: &'static [u8],
    #[values(
        BYTES_OPERATOR_ASSIGN,
        BYTES_OPERATOR_ASSIGN_IF_UNDEFINED,
        BYTES_OPERATOR_ADD,
        BYTES_OPERATOR_REMOVE
    )]
    operator: &'static [u8],
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    post_op_whitespace: &'static [u8],
    #[values(
        (b"A".as_slice(), b"A".as_slice()),
        (b"VALUE".as_slice(), b"VALUE".as_slice()),
        (b"\"VALUE\"".as_slice(), b"VALUE".as_slice()),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice()
        ),
        (
            b"\"Value with \\*\\$\\\"escape\\\" characters\"".as_slice(),
            b"Value with *$\"escape\" characters".as_slice()
        ),
    )]
    (raw_value, ast_value): (&'static [u8], &'static [u8]),
    #[values(
        b"".as_slice(),
        b";".as_slice(),
        b" ;".as_slice(),
    )]
    terminator: &'static [u8],
) {
    use bytes::Bytes;

    use crate::parse::AstParser;

    let expected_ast = Ast::from_iter(vec![match operator {
        BYTES_OPERATOR_ASSIGN => AstEntry::new_assign(ast_key, ast_value),
        BYTES_OPERATOR_ASSIGN_IF_UNDEFINED => AstEntry::new_assign_if_undefined(ast_key, ast_value),
        BYTES_OPERATOR_ADD => AstEntry::new_add(ast_key, ast_value),
        BYTES_OPERATOR_REMOVE => AstEntry::new_remove(ast_key, ast_value),
        _ => panic!(
            "Unexpected operator: {}",
            OsStr::from_bytes(operator).display()
        ),
    }]);

    let ast = AstParser::new().parse(
        raw_key
            .chain(pre_op_whitespace)
            .chain(operator)
            .chain(post_op_whitespace)
            .chain(raw_value)
            .chain(terminator)
            .bytes()
            .collect::<Result<Bytes, std::io::Error>>()
            .expect("Failed to read chain of raw bytes"),
    );
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), expected_ast);
}

#[rstest]
fn parse_key_reset_value(
    #[values(
        (b"A".as_slice(), b"A".as_slice()),
        (b"KEY".as_slice(), b"KEY".as_slice()),
        (b"\"KEY\"".as_slice(), b"KEY".as_slice()),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice()
        ),
        (
            b"\"Key with \\!\\@\\#\\$\\%\\^\\&\\*\\(\\)\\-\\=\\{\\}\\[\\]\\|\\\\\\:\\;\\\"\\'\\<\\>\\,\\.\\?\\/\\~\\` \\\"escape\\\" characters\"".as_slice(),
            b"Key with !@#$%^&*()-={}[]|\\:;\"'<>,.?/~` \"escape\" characters".as_slice()
        ),
    )]
    (raw_key, ast_key): (&'static [u8], &'static [u8]),
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    pre_op_whitespace: &'static [u8],
    #[values(BYTES_OPERATOR_RESET, BYTES_OPERATOR_CLEAR)] operator: &'static [u8],
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    post_op_whitespace: &'static [u8],
    #[values(
        b"".as_slice(),
        b";".as_slice(),
        b" ;".as_slice(),
    )]
    terminator: &'static [u8],
) {
    use bytes::Bytes;

    use crate::parse::AstParser;

    let expected_ast = Ast::from_iter(vec![match operator {
        BYTES_OPERATOR_RESET => AstEntry::new_reset(ast_key),
        BYTES_OPERATOR_CLEAR => AstEntry::new_clear(ast_key),
        _ => panic!(
            "Unexpected operator: {}",
            OsStr::from_bytes(operator).display()
        ),
    }]);

    let ast = AstParser::new().parse(
        raw_key
            .chain(pre_op_whitespace)
            .chain(operator)
            .chain(post_op_whitespace)
            .chain(terminator)
            .bytes()
            .collect::<Result<Bytes, std::io::Error>>()
            .expect("Failed to read chain of raw bytes"),
    );
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), expected_ast);
}

#[rstest]
fn parse_key_group(
    #[values(
        (b"A".as_slice(), b"A".as_slice()),
        (b"KEY".as_slice(), b"KEY".as_slice()),
        (b"\"KEY\"".as_slice(), b"KEY".as_slice()),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789./-:_".as_slice()
        ),
        (
            b"\"Key with \\!\\@\\#\\$\\%\\^\\&\\*\\(\\)\\-\\=\\{\\}\\[\\]\\|\\\\\\:\\;\\\"\\'\\<\\>\\,\\.\\?\\/\\~\\` \\\"escape\\\" characters\"".as_slice(),
            b"Key with !@#$%^&*()-={}[]|\\:;\"'<>,.?/~` \"escape\" characters".as_slice()
        ),
    )]
    (raw_group_key, ast_group_key): (&'static [u8], &'static [u8]),
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    pre_group_op_whitespace: &'static [u8],
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
    )]
    post_group_op_whitespace: &'static [u8],
    #[values(
        b"".as_slice(),
        b" ".as_slice(),
        b"  \t\n".as_slice(),
        b"#\n".as_slice(),
        b"# comment with text\n".as_slice(),
        b"# comment with text\n#second comment   \n".as_slice(),
    )]
    group_inner_whitespace: &'static [u8],
    #[values(
        b"".as_slice(),
        b";".as_slice(),
        b" ;".as_slice(),
    )]
    terminator: &'static [u8],
) {
    use bytes::Bytes;

    use crate::parse::AstParser;

    let expected_ast = Ast::from_iter(vec![AstEntry::new_group(ast_group_key, vec![])]);

    let ast = AstParser::new().parse(
        raw_group_key
            .chain(pre_group_op_whitespace)
            .chain(b":".as_slice())
            .chain(post_group_op_whitespace)
            .chain(b"{".as_slice())
            .chain(group_inner_whitespace)
            .chain(b"}".as_slice())
            .chain(terminator)
            .bytes()
            .collect::<Result<Bytes, std::io::Error>>()
            .expect("Failed to read chain of raw bytes"),
    );
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), expected_ast);
}

#[rstest]
#[case(
        b"KEY=UNQUOTED_STRING",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY_WITH_UNDERSCORES=UNQUOTED_STRING/0123456789.",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY_WITH_UNDERSCORES".to_vec(), b"UNQUOTED_STRING/0123456789.".to_vec())
        ])
    )]
#[case(
        b"KEY=\"QUOTED String 0123456789 \\\\ \\\"\"",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String 0123456789 \\ \"".to_vec())
        ])
    )]
#[case(
        b"KEY=UNQUOTED_STRING;",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY = UNQUOTED_STRING ;",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY\n=\n\t    UNQUOTED_STRING;",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY=UNQUOTED_STRING;KEY2=\"QUOTED String @\";",
        Ast::from_iter(vec![
            AstEntry::new_assign(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY2".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        Ast::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"# comment before a key-value
        KEY+=UNQUOTED_STRING
        KEY=\"QUOTED String @\";
        ",
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING # comment on same line as key-value
        KEY=\"QUOTED String @\";# comment on same line as key-value
        ",
        Ast::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"
        KEY+=UNQUOTED_STRING# comment on same line as key-value
        KEY=\"QUOTED String @\"; # comment on same line as key-value
        ",
        Ast::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY+=UNQUOTED_STRING",
        Ast::from_iter(vec![
            AstEntry::new_add(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY-=UNQUOTED_STRING",
        Ast::from_iter(vec![
            AstEntry::new_remove(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY:=UNQUOTED_STRING",
        Ast::from_iter(vec![
            AstEntry::new_assign_if_undefined(b"KEY".to_vec(), b"UNQUOTED_STRING".to_vec())
        ])
    )]
#[case(
        b"KEY!",
        Ast::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !",
        Ast::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !",
        Ast::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY!
        KEY=\"QUOTED String @\";
        ",
        Ast::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY!NEXT-=VALUE",
        Ast::from_iter(vec![
            AstEntry::new_reset(b"KEY".to_vec()),
            AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
        ])
    )]
#[case(
        b"KEY!!",
        Ast::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !!",
        Ast::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY !!",
        Ast::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec())
        ])
    )]
#[case(
        b"KEY!!
        KEY=\"QUOTED String @\";
        ",
        Ast::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec()),
            AstEntry::new_assign(b"KEY".to_vec(), b"QUOTED String @".to_vec()),
        ])
    )]
#[case(
        b"KEY!!NEXT-=VALUE",
        Ast::from_iter(vec![
            AstEntry::new_clear(b"KEY".to_vec()),
            AstEntry::new_remove(b"NEXT".to_vec(), b"VALUE".to_vec()),
        ])
    )]
fn parse_key_op_value(#[case] input: &[u8], #[case] output: Ast) {
    use crate::parse::AstParser;

    let ast = AstParser::new().parse(input.to_vec());
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), output);
}

#[rstest]
#[case(
        b"KEY: {}",
        Ast::from_iter(vec![
            AstEntry::new_group(b"KEY".to_vec(), vec![])
        ])
    )]
#[case(
        b"      \t_   \n: {\t     }",
        Ast::from_iter(vec![
            AstEntry::new_group(b"_".to_vec(), vec![])
        ])
    )]
#[case(
        b"KEY: {
            PART1 = VALUE;
            PART2 = other
        }",
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
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
        Ast::from_iter(vec![
            AstEntry::new_group(
                b"KEY".to_vec(),
                vec![
                    AstEntry::new_assign(b"PART1".to_vec(), b"VALUE".to_vec()),
                    AstEntry::new_assign(b"PART2".to_vec(), b"other".to_vec()),
                ]
            )
        ])
    )]
fn parse_group(#[case] input: &[u8], #[case] output: Ast) {
    let ast = AstParser::new().parse(input.to_vec());
    assert!(ast.is_ok(), "error converting input to tree: {ast:?}");
    assert_eq!(ast.unwrap(), output);
}
