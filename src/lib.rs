use std::{collections::BTreeMap, ops::Deref, str::Chars};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag},
    character::complete::{char, one_of, space0},
    combinator::{all_consuming, map, map_opt, map_res, opt, recognize, value},
    multi::{many0, many1, separated_list1},
    sequence::{delimited, separated_pair, tuple},
    IResult,
};
use unic_normal::{Decompositions, StrNormalForm};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct CollationElement {
    variable: bool,
    symbol: u16,
    diacritic: u16,
    case: u16,
}

// * Parse the table
// * Normalize each string
//   * Convert to nfd (unic-normal)
// * Construct collation element array
//   * Find longest initial substring S that has a match in the collation table
//     * If there are non-starters following S, process each non-starter C
//     * If C is an unblocked non-starter with respect to S, find if S+C has a match in the collation table
//     * If there is a match remove S by S+C and remove C
//   * Fetch the corresponding value from the table, else synthesize one (Derived Collation Elements)
//   * Process collation elements according to variable weight setting
//   * Append collation elements to the collation element array
//   * Proceed to next point past S
// * Construct a sort key by successively appending all non-zero weights from the collation element array
//   * For each weight L in the collation element array from 1 to the maximum level
//     * If L is not 1, append a level separator
//     * If the collation element is forwards at level L
//       * For each collation element CE in the array
//         * Append CE_L to the sort key if CE_L != 0
//     * Else the collation element is backwards at level L
//       * Form a list of all non-zero CE_L values
//       * Reverse that list
//       * Append the CE_L values from that list to the sort key
// * Compare the keys, easy peasy

// Default Unicode Collation Element Table
static DUCET: &'static str = include_str!("../external/allkeys.txt");

fn take_sep(i: &str) -> IResult<&str, ()> {
    let (i, _) = separated_pair(space0, char(';'), space0)(i)?;
    Ok((i, ()))
}

fn parse_element(i: &str) -> IResult<&str, String> {
    map(separated_list1(char(' '), parse_char), |v| {
        v.into_iter().collect::<String>()
    })(i)
}

fn parse_variable(i: &str) -> IResult<&str, bool> {
    let (i, c) = alt((tag("*"), tag(".")))(i)?;
    Ok((i, c == "*"))
}

fn parse_sortkey(i: &str) -> IResult<&str, CollationElement> {
    let (i, (var, levels)) = delimited(
        tag("["),
        tuple((parse_variable, separated_list1(tag("."), parse_hex))),
        tag("]"),
    )(i)?;
    if levels.len() == 3 {
        Ok((
            i,
            CollationElement {
                variable: var,
                symbol: levels[0],
                diacritic: levels[1],
                case: levels[2],
            },
        ))
    } else {
        Err(nom::Err::Incomplete(nom::Needed::Unknown))
    }
}

fn parse_hex(i: &str) -> IResult<&str, u16> {
    map_res(
        recognize(many1(one_of("0123456789abcdefABCDEF"))),
        |out: &str| u16::from_str_radix(out, 16),
    )(i)
}

fn parse_char(i: &str) -> IResult<&str, char> {
    map_opt(
        recognize(many1(one_of("0123456789abcdefABCDEF"))),
        |out: &str| {
            u32::from_str_radix(out, 16)
                .ok()
                .and_then(|u| char::from_u32(u))
        },
    )(i)
}

fn parse_row(i: &str) -> IResult<&str, (String, Vec<CollationElement>)> {
    let (i, char_points) = parse_element(i)?;
    let (i, _) = take_sep(i)?;
    let (i, key) = many1(parse_sortkey)(i)?;
    let (i, _) = tuple((many0(char(' ')), char('#'), is_not("\n"), tag("\n")))(i)?;
    Ok((i, (char_points, key)))
}

pub struct CollationElementTable {
    data: BTreeMap<String, Vec<CollationElement>>,
}

impl CollationElementTable {
    pub fn from(i: &str) -> Result<Self, nom::Err<nom::error::Error<&str>>> {
        let mut data = BTreeMap::new();
        let (i, _) = all_consuming(many1(alt((
            // Empty line
            value((), tag("\n")),
            // A comment
            value(
                (),
                tuple((space0, char('#'), opt(is_not("\n")), char('\n'))),
            ),
            value((), tuple((tag("@version"), is_not("\n"), char('\n')))),
            // TODO: Implicit weight and version
            value(
                (),
                tuple((tag("@implicitweights"), is_not("\n"), char('\n'))),
            ),
            // A row in the table
            map(parse_row, |(char_points, key)| {
                data.insert(char_points, key);
            }),
        ))))(i)?;
        println!("{}", i);
        Ok(Self { data })
    }
}

impl Deref for CollationElementTable {
    type Target = BTreeMap<String, Vec<CollationElement>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

struct CollationElements<'a> {
    normalized: std::iter::Peekable<Decompositions<Chars<'a>>>,
    table: &'a CollationElementTable,
}

impl<'a> CollationElements<'a> {
    fn from(table: &'a CollationElementTable, s: &'a str) -> Self {
        let normalized = s.nfd();
        Self {
            table,
            normalized: normalized.peekable(),
        }
    }
}

// * Construct collation element array
//   * Find longest initial substring S that has a match in the collation table
//     * If there are non-starters following S, process each non-starter C
//     * If C is an unblocked non-starter with respect to S, find if S+C has a match in the collation table
//     * If there is a match remove S by S+C and remove C
//   * Fetch the corresponding value from the table, else synthesize one (Derived Collation Elements)
//   * Process collation elements according to variable weight setting
//   * Append collation elements to the collation element array
//   * Proceed to next point past S
impl<'a> Iterator for CollationElements<'a> {
    type Item = Vec<CollationElement>;

    fn next(&mut self) -> Option<Self::Item> {
        // OPTIMIZE: Remove allocations and copying
        let mut s = String::from(self.normalized.next()?);
        let mut elem = self.table.get(&s)?;
        while let Some(&c) = self.normalized.peek() {
            s.push(c);
            if let Some(e) = self.table.get(&s) {
                elem = e;
                self.normalized.next();
            } else {
                s.pop();
                break;
            }
        }
        Some(elem.clone())
    }
}

#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortKey {
    symbols: Vec<u16>,
    diacritics: Vec<u16>,
    cases: Vec<u16>,
}

impl SortKey {
    fn new() -> Self {
        Self::default()
    }
}

// This skips the third step
pub fn generate_sort_key(s: &str, table: &CollationElementTable) -> SortKey {
    let mut key = SortKey::new();
    for elem in CollationElements::from(&table, s).flatten() {
        if elem.symbol != 0 {
            key.symbols.push(elem.symbol);
        } else {
            continue;
        }

        if elem.diacritic != 0 {
            key.diacritics.push(elem.diacritic);
        } else {
            continue;
        }

        if elem.case != 0 {
            key.cases.push(elem.case)
        }
    }
    key
}

#[cfg(unix)]
mod test {
    use super::*;

    // Just to make sure it parses without any errors.
    #[test]
    fn just_print() {
        let _table = CollationElementTable::from(DUCET).unwrap();
    }

    #[test]
    fn ascii_strings() {
        let table = CollationElementTable::from(DUCET).unwrap();

        let mut v = ["a", "b", "C", "A", "c", "B"];
        v.sort_by_key(|s| generate_sort_key(s, &table));
        assert_eq!(v, ["a", "A", "b", "B", "c", "C"]);
    }
}
