# Collate
Implementation of [UTS #10](https://www.unicode.org/reports/tr10/), Unicode
Collation Algorithm, in pure (and safe) Rust. It is currently a work in
progress. The goal is to provide a fast and reliable algorithm that can be
easily customized for particular usecases.

PR's of all kinds are welcome.

## Planned Features
This library is a work in progress. The checked features are implemented while
the others are not.

- [ ] Basic functionality
  - [x] Parse Default Unicode Collation Element Table (DUCET).
  - [ ] Parse DUCET at build time
  - [x] Generate sort keys from `&str` using the table.
  - [ ] Generate sort keys from implicit weights
  - [ ] Handling of invalid unicode
- [ ] Tailoring
  - [ ] Parse `allkeys_CLDR.txt`
  - [ ] Parsing collation Tailoring syntax
    - [ ] Surrounding XML
    - [x] Settings in `[key value]` format
    - [x] Operators: `<`, `<<`, `<<<`, `<<<<`, `&` and `=`
    - [ ] Operators: `<*`, `<<*`, `<<<*`, `<<<<*` and `=*`
    - [x] Escaped characters
    - [ ] Extensions (`/`)
    - [ ] Prefixes (`|`)
    - [ ] Comments (`#`)
  - [ ] Applying the parsed rules to `allkeys_CLDR.txt`
  - [ ] Applying the settings

## License
Currently still undecided. It is probably going to be either Apache 2.0, MIT,
BSD or Unlicense.
