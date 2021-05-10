use crate::CollationElement;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, hex_digit1, line_ending, not_line_ending, space0},
    combinator::{all_consuming, map, map_opt, map_res, opt, value},
    multi::{many1, separated_list1},
    sequence::{delimited, separated_pair, terminated, tuple},
    IResult,
};
use std::collections::BTreeMap;

pub fn table<'a>(
    i: &'a str,
    data: &mut BTreeMap<String, Vec<CollationElement>>,
) -> IResult<&'a str, ()> {
    value(
        (),
        all_consuming(many1(alt((
            // Empty line
            value((), tag("\n")),
            // A comment
            value(
                (),
                tuple((space0, char('#'), opt(is_not("\n")), char('\n'))),
            ),
            // TODO: Implicit weight and version
            value((), tuple((tag("@version"), is_not("\n"), char('\n')))),
            value(
                (),
                tuple((tag("@implicitweights"), is_not("\n"), char('\n'))),
            ),
            // A row in the table
            map(row, |(char_points, key)| {
                data.insert(char_points, key);
            }),
        )))),
    )(i)
}

fn row(i: &str) -> IResult<&str, (String, Vec<CollationElement>)> {
    terminated(separated_pair(element, sep, many1(sortkey)), opt(comment))(i)
}

fn comment(i: &str) -> IResult<&str, ()> {
    value((), tuple((char('#'), not_line_ending, line_ending)))(i)
}

fn element(i: &str) -> IResult<&str, String> {
    map(separated_list1(char(' '), code_point), |v| {
        v.into_iter().collect::<String>()
    })(i)
}

fn code_point(i: &str) -> IResult<&str, char> {
    map_opt(hex_digit1, |out: &str| {
        u32::from_str_radix(out, 16)
            .ok()
            .and_then(|u| char::from_u32(u))
    })(i)
}

fn sep(i: &str) -> IResult<&str, ()> {
    value((), separated_pair(space0, char(';'), space0))(i)
}

fn sortkey(i: &str) -> IResult<&str, CollationElement> {
    let (i, (var, levels)) = delimited(
        char('['),
        tuple((variable, separated_list1(char('.'), hex))),
        char(']'),
    )(i)?;
    if levels.len() == 3 {
        Ok((
            i,
            CollationElement {
                variable: var,
                primary: levels[0],
                secondary: levels[1],
                tertiary: levels[2],
            },
        ))
    } else {
        Err(nom::Err::Incomplete(nom::Needed::Unknown))
    }
}

fn variable(i: &str) -> IResult<&str, bool> {
    alt((value(true, char('*')), value(false, char('.'))))(i)
}

fn hex(i: &str) -> IResult<&str, u16> {
    map_res(hex_digit1, |out: &str| u16::from_str_radix(out, 16))(i)
}
