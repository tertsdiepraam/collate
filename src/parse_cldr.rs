use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::{
            alphanumeric1, anychar, char, multispace0, multispace1, none_of, one_of, satisfy,
            space1,
        },
        is_hex_digit,
    },
    combinator::{all_consuming, map, map_opt, opt, recognize, value},
    multi::{count, many0, many1, many_m_n, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

#[derive(Eq, PartialEq, Debug)]
struct RuleCommand {
    command: RuleCommandType,
    sequence: String,
}

#[derive(Eq, PartialEq, Debug, Clone)]
enum RuleCommandType {
    SetContext {
        before: Option<u8>,
    },
    Equal,
    Increment {
        level: u8,
        prefix: Option<String>,
        extension: Option<String>,
    },
}

pub fn cldr(i: &str) -> IResult<&str, ()> {
    all_consuming(delimited(
        multispace0,
        separated_pair(settings, multispace0, rules),
        multispace0,
    ))(i)?;
    Ok((i, ()))
}

fn settings(i: &str) -> IResult<&str, Vec<(&str, &str)>> {
    separated_list0(multispace0, setting)(i)
}

// [key value]
fn setting(i: &str) -> IResult<&str, (&str, &str)> {
    delimited(
        char('['),
        separated_pair(identifier, space1, identifier),
        char(']'),
    )(i)
}

fn identifier(i: &str) -> IResult<&str, &str> {
    recognize(many1(alt((alphanumeric1, tag("-")))))(i)
}

fn rules(i: &str) -> IResult<&str, Vec<RuleCommand>> {
    map(many0(rule), |v| v.into_iter().flatten().collect())(i)
}

fn rule(i: &str) -> IResult<&str, Vec<RuleCommand>> {
    map(
        preceded(multispace0, alt((increment, equal, set_context))),
        |command| vec![command],
    )(i)
}

fn increment(i: &str) -> IResult<&str, RuleCommand> {
    let (i, (level, sequence, prefix, extension)) = tuple((
        map(many_m_n(1, 4, char('<')), |s| s.len() as u8),
        preceded(multispace0, sequence),
        opt(preceded(
            tuple((multispace0, char('|'), multispace0)),
            sequence,
        )),
        opt(preceded(
            tuple((multispace0, char('/'), multispace0)),
            sequence,
        )),
    ))(i)?;
    Ok((
        i,
        RuleCommand {
            command: RuleCommandType::Increment {
                level,
                prefix,
                extension,
            },
            sequence,
        },
    ))
}

fn set_context(i: &str) -> IResult<&str, RuleCommand> {
    map(
        preceded(
            pair(char('&'), multispace0),
            pair(opt(terminated(before, multispace0)), sequence),
        ),
        |(before, sequence)| RuleCommand {
            command: RuleCommandType::SetContext { before },
            sequence,
        },
    )(i)
}

fn before(i: &str) -> IResult<&str, u8> {
    delimited(
        char('['),
        preceded(
            pair(tag("before"), multispace1),
            map(one_of("123"), |c| c.to_digit(10).unwrap() as u8),
        ),
        char(']'),
    )(i)
}

fn equal(i: &str) -> IResult<&str, RuleCommand> {
    map(preceded(pair(char('='), multispace0), sequence), |s| {
        RuleCommand {
            command: RuleCommandType::Equal,
            sequence: s,
        }
    })(i)
}

fn is_reserved_char(c: char) -> bool {
    c.is_whitespace()
        || (c >= '\u{0021}' && c <= '\u{002f}')
        || (c >= '\u{003A}' && c <= '\u{0040}')
        || (c >= '\u{005B}' && c <= '\u{0060}')
        || (c >= '\u{007B}' && c <= '\u{007E}')
}

fn sequence(i: &str) -> IResult<&str, String> {
    map(
        many1(alt((
            map(
                recognize(many1(satisfy(|c| !is_reserved_char(c)))),
                |s: &str| s.to_owned(),
            ),
            quoted_chars,
        ))),
        |v| v.into_iter().collect(),
    )(i)
}

fn quoted_chars(i: &str) -> IResult<&str, String> {
    delimited(
        char('\''),
        many1(alt((none_of(r"\'"), escaped_char))),
        char('\''),
    )(i)
    .map(|(i, v)| (i, v.iter().collect()))
}

/// Parses `\uhhhh` or `\U00hhhhhh` or other escaped characters
/// Should roughly match the behaviour of [icu::UnicodeString::unescape](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/classicu_1_1UnicodeString.html#a330aa00f6ab316d3f7bbe1331c084d15)
/// 100% compatibility is not necessary if no-locale uses certain escape sequences
fn escaped_char(i: &str) -> IResult<&str, char> {
    preceded(
        char('\\'),
        alt((
            preceded(char('U'), hex_digits(8)),
            preceded(char('u'), hex_digits(4)),
            value('\u{7}', char('a')),
            value('\u{8}', char('b')),
            value('\t', char('t')),
            value('\n', char('n')),
            value('\u{B}', char('v')),
            value('\u{C}', char('f')),
            value('\u{D}', char('r')),
            value('\u{1B}', char('e')),
            value('\u{22}', char('"')),
            value('\u{27}', char('\'')),
            value('\u{3F}', char('?')),
            value('\u{5C}', char('\\')),
            anychar,
        )),
    )(i)
}

// A specific number of hex digits turned parsed into a char
fn hex_digits(n: u8) -> impl Fn(&str) -> IResult<&str, char> {
    move |i: &str| {
        map_opt(
            recognize(count(
                satisfy(|c| c < u8::MAX as char && is_hex_digit(c as u8)),
                n as usize,
            )),
            |out: &str| {
                u32::from_str_radix(out, 16)
                    .ok()
                    .and_then(|u| char::from_u32(u))
            },
        )(i)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_escaped_char() {
        assert_eq!(escaped_char(r"\u012345"), Ok(("45", '\u{0123}')));
        assert_eq!(escaped_char(r"\U00012345"), Ok(("", '\u{012345}')));
    }

    #[test]
    fn test_quoted_chars() {
        assert_eq!(
            quoted_chars(r"'\u1111\u2222\U00000101' some other text"),
            Ok((" some other text", "\u{1111}\u{2222}\u{101}".into()))
        );

        assert_eq!(
            quoted_chars(r"' \u1111hello<'"),
            Ok(("", " \u{1111}hello<".into()))
        );
    }

    #[test]
    fn test_sequence() {
        assert_eq!(
            sequence(r"hello'\u1111 \''world"),
            Ok(("", "hello\u{1111} 'world".into()))
        );

        assert_eq!(sequence("hello world"), Ok((" world", "hello".into())));
    }

    #[test]
    fn test_single_rules() {
        assert_eq!(
            rule("& a"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::SetContext { before: None },
                    sequence: "a".into()
                }]
            ))
        );

        assert_eq!(
            rule("< a"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::Increment {
                        level: 1,
                        prefix: None,
                        extension: None
                    },
                    sequence: "a".into()
                }]
            ))
        );
    }

    #[test]
    fn test_rules() {
        assert_eq!(
            rules("& a < b"),
            Ok((
                "",
                vec![
                    RuleCommand {
                        command: RuleCommandType::SetContext { before: None },
                        sequence: "a".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Increment {
                            level: 1,
                            prefix: None,
                            extension: None
                        },
                        sequence: "b".into()
                    },
                ]
            ))
        );

        assert_eq!(
            rules("& a < b\n<< c\n\t\t\t\t<<<\nd <<<< e = f"),
            Ok((
                "",
                vec![
                    RuleCommand {
                        command: RuleCommandType::SetContext { before: None },
                        sequence: "a".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Increment {
                            level: 1,
                            prefix: None,
                            extension: None
                        },
                        sequence: "b".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Increment {
                            level: 2,
                            prefix: None,
                            extension: None
                        },
                        sequence: "c".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Increment {
                            level: 3,
                            prefix: None,
                            extension: None
                        },
                        sequence: "d".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Increment {
                            level: 4,
                            prefix: None,
                            extension: None
                        },
                        sequence: "e".into()
                    },
                    RuleCommand {
                        command: RuleCommandType::Equal,
                        sequence: "f".into()
                    }
                ]
            ))
        );
    }

    #[test]
    fn test_prefix_and_extension() {
        assert_eq!(
            rule("<<< ab | cd / ef"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::Increment {
                        level: 3,
                        prefix: Some("cd".into()),
                        extension: Some("ef".into()),
                    },
                    sequence: "ab".into(),
                }]
            )),
        );

        assert_eq!(
            rule("<<< ab|cd/ef"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::Increment {
                        level: 3,
                        prefix: Some("cd".into()),
                        extension: Some("ef".into()),
                    },
                    sequence: "ab".into(),
                }]
            )),
        );

        assert_eq!(
            rule("<<ab|cd"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::Increment {
                        level: 2,
                        prefix: Some("cd".into()),
                        extension: None,
                    },
                    sequence: "ab".into(),
                }]
            )),
        );

        assert_eq!(
            rule("<<ab/cd"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::Increment {
                        level: 2,
                        prefix: None,
                        extension: Some("cd".into()),
                    },
                    sequence: "ab".into(),
                }]
            )),
        )
    }

    #[test]
    fn test_before() {
        assert_eq!(before("[before 2]"), Ok(("", 2)),);

        assert_eq!(
            rule("&[before 2] a"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::SetContext { before: Some(2) },
                    sequence: "a".into()
                }]
            ))
        );

        assert_eq!(
            rule("&    [before      1] a"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::SetContext { before: Some(1) },
                    sequence: "a".into()
                }]
            ))
        );

        assert_eq!(
            rule("&[before 3]a"),
            Ok((
                "",
                vec![RuleCommand {
                    command: RuleCommandType::SetContext { before: Some(3) },
                    sequence: "a".into()
                }]
            ))
        );
    }
}
