use std::ops::RangeInclusive;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        complete::{
            alphanumeric1, anychar, char, line_ending, multispace0, multispace1, none_of,
            not_line_ending, one_of, satisfy, space1,
        },
        is_hex_digit,
    },
    combinator::{all_consuming, map, map_opt, opt, recognize, value},
    multi::{count, many0, many1, many_m_n, separated_list0},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct Collation {
    pub(crate) r#type: String,
    pub(crate) rules: CollationRules,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct CollationRules {
    pub(crate) settings: Vec<(String, String)>,
    pub(crate) rules: Vec<Rule>,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Rule {
    SetContext {
        before: Option<u8>,
        sequence: String,
    },
    Equal {
        sequence: String,
    },
    MultiEqual {
        multisequence: Vec<SequenceElement>,
    },
    Increment {
        level: u8,
        prefix: Option<String>,
        extension: Option<String>,
        sequence: String,
    },
    MultiIncrement {
        level: u8,
        multisequence: Vec<SequenceElement>,
    },
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum SequenceElement {
    Range(RangeInclusive<char>),
    Char(char),
}

pub fn cldr<'a>(i: &'a str) -> Result<CollationRules, nom::Err<nom::error::Error<&'a str>>> {
    match map(
        all_consuming(delimited(
            comment,
            separated_pair(settings, comment, rules),
            comment,
        )),
        |(settings, rules)| CollationRules { settings, rules },
    )(i)
    {
        Ok((_, col)) => Ok(col),
        Err(x) => Err(x),
    }
}

fn settings(i: &str) -> IResult<&str, Vec<(String, String)>> {
    separated_list0(comment, setting)(i)
}

// [key value]
fn setting(i: &str) -> IResult<&str, (String, String)> {
    delimited(
        char('['),
        separated_pair(
            map(identifier, |s| s.into()),
            space1,
            map(identifier, |s| s.into()),
        ),
        char(']'),
    )(i)
}

fn identifier(i: &str) -> IResult<&str, &str> {
    recognize(many1(alt((alphanumeric1, tag("-")))))(i)
}

fn rules(i: &str) -> IResult<&str, Vec<Rule>> {
    many0(rule)(i)
}

fn rule(i: &str) -> IResult<&str, Rule> {
    preceded(
        comment,
        alt((multi_increment, increment, multi_equal, equal, set_context)),
    )(i)
}

fn multi_increment(i: &str) -> IResult<&str, Rule> {
    let (i, (level, multisequence)) = separated_pair(
        map(many_m_n(1, 4, char('<')), |s| s.len() as u8),
        char('*'),
        preceded(comment, multisequence),
    )(i)?;
    Ok((
        i,
        Rule::MultiIncrement {
            level,
            multisequence,
        },
    ))
}

fn increment(i: &str) -> IResult<&str, Rule> {
    let (i, (level, sequence, prefix, extension)) = tuple((
        map(many_m_n(1, 4, char('<')), |s| s.len() as u8),
        preceded(comment, sequence),
        opt(preceded(tuple((comment, char('|'), comment)), sequence)),
        opt(preceded(tuple((comment, char('/'), comment)), sequence)),
    ))(i)?;
    Ok((
        i,
        Rule::Increment {
            level,
            prefix,
            extension,
            sequence,
        },
    ))
}

fn set_context(i: &str) -> IResult<&str, Rule> {
    map(
        preceded(
            pair(char('&'), comment),
            pair(opt(terminated(before, comment)), sequence),
        ),
        |(before, sequence)| Rule::SetContext { before, sequence },
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

fn multi_equal(i: &str) -> IResult<&str, Rule> {
    map(
        preceded(pair(tag("=*"), comment), multisequence),
        |multisequence| Rule::MultiEqual { multisequence },
    )(i)
}

fn equal(i: &str) -> IResult<&str, Rule> {
    map(preceded(pair(char('='), comment), sequence), |sequence| {
        Rule::Equal { sequence }
    })(i)
}

fn is_reserved_char(c: char) -> bool {
    c.is_whitespace()
        || (c >= '\u{0021}' && c <= '\u{002f}')
        || (c >= '\u{003A}' && c <= '\u{0040}')
        || (c >= '\u{005B}' && c <= '\u{0060}')
        || (c >= '\u{007B}' && c <= '\u{007E}')
}

fn legal_char(i: &str) -> IResult<&str, char> {
    satisfy(|c| !is_reserved_char(c))(i)
}

fn multisequence(i: &str) -> IResult<&str, Vec<SequenceElement>> {
    many1(alt((
        map(
            separated_pair(legal_char, char('-'), legal_char),
            |(beg, end)| SequenceElement::Range(beg..=end),
        ),
        map(legal_char, |c| SequenceElement::Char(c)),
    )))(i)
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

// Matches whitespace, optionally with a comment
fn comment(i: &str) -> IResult<&str, ()> {
    delimited(
        multispace0,
        value((), opt(tuple((char('#'), not_line_ending, line_ending)))),
        multispace0,
    )(i)
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
                Rule::SetContext {
                    before: None,
                    sequence: "a".into()
                },
            ))
        );

        assert_eq!(
            rule("< a"),
            Ok((
                "",
                Rule::Increment {
                    level: 1,
                    prefix: None,
                    extension: None,
                    sequence: "a".into()
                }
            ))
        );

        assert_eq!(
            rule("<* abc-z"),
            Ok((
                "",
                Rule::MultiIncrement {
                    level: 1,
                    multisequence: vec![
                        SequenceElement::Char('a'),
                        SequenceElement::Char('b'),
                        SequenceElement::Range('c'..='z'),
                    ]
                }
            ))
        );

        assert_eq!(
            rule("=* abc-z"),
            Ok((
                "",
                Rule::MultiEqual {
                    multisequence: vec![
                        SequenceElement::Char('a'),
                        SequenceElement::Char('b'),
                        SequenceElement::Range('c'..='z'),
                    ]
                }
            ))
        )
    }

    #[test]
    fn test_rules() {
        assert_eq!(
            rules("& a < b"),
            Ok((
                "",
                vec![
                    Rule::SetContext {
                        before: None,
                        sequence: "a".into(),
                    },
                    Rule::Increment {
                        level: 1,
                        prefix: None,
                        extension: None,
                        sequence: "b".into(),
                    },
                ]
            ))
        );

        assert_eq!(
            rules("& a < b\n<< c\n\t\t\t\t<<<\nd <<<< e = f"),
            Ok((
                "",
                vec![
                    Rule::SetContext {
                        before: None,
                        sequence: "a".into(),
                    },
                    Rule::Increment {
                        level: 1,
                        prefix: None,
                        extension: None,
                        sequence: "b".into(),
                    },
                    Rule::Increment {
                        level: 2,
                        prefix: None,
                        extension: None,
                        sequence: "c".into(),
                    },
                    Rule::Increment {
                        level: 3,
                        prefix: None,
                        extension: None,
                        sequence: "d".into(),
                    },
                    Rule::Increment {
                        level: 4,
                        prefix: None,
                        extension: None,
                        sequence: "e".into(),
                    },
                    Rule::Equal {
                        sequence: "f".into(),
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
                Rule::Increment {
                    level: 3,
                    prefix: Some("cd".into()),
                    extension: Some("ef".into()),
                    sequence: "ab".into(),
                }
            )),
        );

        assert_eq!(
            rule("<<< ab|cd/ef"),
            Ok((
                "",
                Rule::Increment {
                    level: 3,
                    prefix: Some("cd".into()),
                    extension: Some("ef".into()),
                    sequence: "ab".into(),
                }
            )),
        );

        assert_eq!(
            rule("<<ab|cd"),
            Ok((
                "",
                Rule::Increment {
                    level: 2,
                    prefix: Some("cd".into()),
                    extension: None,
                    sequence: "ab".into(),
                }
            )),
        );

        assert_eq!(
            rule("<<ab/cd"),
            Ok((
                "",
                Rule::Increment {
                    level: 2,
                    prefix: None,
                    extension: Some("cd".into()),
                    sequence: "ab".into(),
                }
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
                Rule::SetContext {
                    before: Some(2),
                    sequence: "a".into(),
                }
            ))
        );

        assert_eq!(
            rule("&    [before      1] a"),
            Ok((
                "",
                Rule::SetContext {
                    before: Some(1),
                    sequence: "a".into(),
                }
            ))
        );

        assert_eq!(
            rule("&[before 3]a"),
            Ok((
                "",
                Rule::SetContext {
                    before: Some(3),
                    sequence: "a".into(),
                }
            ))
        );
    }

    #[test]
    fn test_comment() {
        assert_eq!(
            rule("<< # comment 1\n   ab  # comment 2\n/#comment 3\ncd"),
            Ok((
                "",
                Rule::Increment {
                    level: 2,
                    prefix: None,
                    extension: Some("cd".into()),
                    sequence: "ab".into(),
                }
            )),
        )
    }
}
