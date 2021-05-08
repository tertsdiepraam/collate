/// Steps of the UCA algorithm
/// * Parse the table
/// * Normalize each string
///   * Convert to nfd (unic-normal)
/// * Construct collation element array
///   * Find longest initial substring S that has a match in the collation table
///     * If there are non-starters following S, process each non-starter C
///     * If C is an unblocked non-starter with respect to S, find if S+C has a match in the collation table
///     * If there is a match remove S by S+C and remove C
///   * Fetch the corresponding value from the table, else synthesize one (Derived Collation Elements)
///   * Process collation elements according to variable weight setting
///   * Append collation elements to the collation element array
///   * Proceed to next point past S
/// * Construct a sort key by successively appending all non-zero weights from the collation element array
///   * For each weight L in the collation element array from 1 to the maximum level
///     * If L is not 1, append a level separator
///     * If the collation element is forwards at level L
///       * For each collation element CE in the array
///         * Append CE_L to the sort key if CE_L != 0
///     * Else the collation element is backwards at level L
///       * Form a list of all non-zero CE_L values
///       * Reverse that list
///       * Append the CE_L values from that list to the sort key
/// * Compare the keys, easy peasy
mod parse;
use std::{cmp::Ordering, collections::BTreeMap, iter::Peekable, ops::Deref, str::Chars};

use unic_normal::{Decompositions, StrNormalForm};

// Default Unicode Collation Element Table
static DUCET: &'static str = include_str!("../external/allkeys.txt");

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct CollationElement {
    variable: bool,
    primary: u16,
    secondary: u16,
    tertiary: u16,
}

pub struct CollationElementTable {
    data: BTreeMap<String, Vec<CollationElement>>,
}

impl CollationElementTable {
    pub fn from(i: &str) -> Result<Self, nom::Err<nom::error::Error<&str>>> {
        let mut data = BTreeMap::new();
        parse::table(i, &mut data)?;
        Ok(Self { data })
    }

    pub fn generate_sort_key(&self, s: &str) -> SortKey {
        let mut key = SortKey::new();
        for elem in CollationElements::from(self, s).flatten() {
            if elem.primary != 0 {
                key.primary.push(elem.primary);
            }
            if elem.secondary != 0 {
                key.secondary.push(elem.secondary);
            }
            if elem.tertiary != 0 {
                key.tertiary.push(elem.tertiary)
            }
        }
        key
    }
}

impl Deref for CollationElementTable {
    type Target = BTreeMap<String, Vec<CollationElement>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

struct CollationElements<'a> {
    normalized: Peekable<Decompositions<Chars<'a>>>,
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

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SortKey {
    primary: Vec<u16>,
    secondary: Vec<u16>,
    tertiary: Vec<u16>,
}

impl SortKey {
    fn new() -> Self {
        Self::default()
    }

    fn iter(&self) -> impl Iterator<Item = &u16> {
        self.primary
            .iter()
            .chain(std::iter::once(&0u16))
            .chain(self.secondary.iter())
            .chain(std::iter::once(&0u16))
            .chain(self.tertiary.iter())
    }
}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl Ord for SortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

#[cfg(unix)]
mod test {
    use super::*;

    #[test]
    fn ascii_strings() {
        let table = CollationElementTable::from(DUCET).unwrap();

        // Casing has low precedence
        let mut v = ["a", "b", "C", "A", "c", "B"];
        v.sort_by_key(|s| table.generate_sort_key(s));
        assert_eq!(v, ["a", "A", "b", "B", "c", "C"]);

        // Casing has lower precedence than letters
        let mut v = ["aaa", "aab", "aAa", "aAb", "aaA", "aaB"];
        v.sort_by_key(|s| table.generate_sort_key(s));
        assert_eq!(v, ["aaa", "aaA", "aAa", "aab", "aaB", "aAb"]);

        // Some real-world filenames typical in a Rust project
        let mut v = [
            "target",
            "Cargo.lock",
            "docs",
            "README.md",
            "Cargo.toml",
            "LICENSE",
            "benches",
            "CONTRIBUTING.md",
            "util",
            "build.rs",
            "DEVELOPER_INSTRUCTIONS.md",
            "CODE_OF_CONDUCT.md",
            "tests",
            "src",
            "examples",
        ];

        v.sort_by_key(|s| table.generate_sort_key(s));

        assert_eq!(
            v,
            [
                "benches",
                "build.rs",
                "Cargo.lock",
                "Cargo.toml",
                "CODE_OF_CONDUCT.md",
                "CONTRIBUTING.md",
                "DEVELOPER_INSTRUCTIONS.md",
                "docs",
                "examples",
                "LICENSE",
                "README.md",
                "src",
                "target",
                "tests",
                "util",
            ]
        );
    }

    #[test]
    fn diacritics() {
        let table = CollationElementTable::from(DUCET).unwrap();

        let mut v = ["cab", "dab", "Cab", "cáb"];
        v.sort_by_key(|s| table.generate_sort_key(s));
        assert_eq!(v, ["cab", "Cab", "cáb", "dab"]);

        let mut v = ["e", "A", "á", "a", "E", "Á", "é", "É"];
        v.sort_by_key(|s| table.generate_sort_key(s));
        assert_eq!(v, ["a", "A", "á", "Á", "e", "E", "é", "É"]);
    }
}
