# Collate
Implementation of [UTS #10](https://www.unicode.org/reports/tr10/), Unicode Collation Algorithm, in pure (and safe) Rust. It is currently a work in progress.

## Planned Features
This library is a work in progress. The checked features are implemented while the others are not.

- [ ] Basic functionality
  - [x] Read and parse Default Unicode Collation Element Table (DUCET).
  - [ ] Parse DUCET at build time
  - [x] Generate sort keys from `&str` using the table.
  - [ ] Generate sort keys from implicit weights
  - [ ] Handling of invalid UTF-8
- [ ] Tailoring
  - [ ] Customizable sort order
  - [ ] Customizable sort level
  - [ ] Locale-specific sort orders based on CLDR data.
    - [ ] Parse CLDR data
    - [ ] Math locale string to data
    - [ ] Adapt DUCET to locale

## License
Currently still undecided.
