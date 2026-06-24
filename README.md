# price-parser

[![All Contributors](https://img.shields.io/badge/all_contributors-1-orange.svg?style=flat-square)](#contributors-)

[![crates.io](https://img.shields.io/crates/v/price-parser.svg)](https://crates.io/crates/price-parser)
[![docs.rs](https://docs.rs/price-parser/badge.svg)](https://docs.rs/price-parser)
[![CI](https://github.com/trananhtung/price-parser/actions/workflows/ci.yml/badge.svg)](https://github.com/trananhtung/price-parser/actions/workflows/ci.yml)
[![license](https://img.shields.io/crates/l/price-parser.svg)](#license)

**Extract a price amount and currency from a string.**

`price-parser` recovers the numeric amount and currency from messy text — `"$1,234.56"`,
`"1.234,56 €"`, `"Rs. 99"`, `"USD 1,000"` — guessing the decimal and thousands separators
along the way. Ideal for web scraping and cleaning up user input.

A faithful Rust port of the [`price-parser`](https://pypi.org/project/price-parser/) Python
package by Zyte (scrapinghub).

- Detects 400+ currency symbols and codes (`$`, `€`, `£`, `EUR`, `Rs`, `руб`, …)
- Guesses the decimal vs thousands separator (`1,234.56` and `1.234,56` both → `1234.56`)
- Differential-tested against the reference `price-parser` implementation

## Install

```toml
[dependencies]
price-parser = "0.1"
```

## Usage

```rust
use price_parser::Price;

let p = Price::from_string("$1,234.56");
assert_eq!(p.amount, Some(1234.56));
assert_eq!(p.currency.as_deref(), Some("$"));

// European convention (dot thousands, comma decimal) is detected:
let p = Price::from_string("1.234,56 EUR");
assert_eq!(p.amount, Some(1234.56));
assert_eq!(p.currency.as_deref(), Some("EUR"));

// Amount without a recognized currency:
let p = Price::from_string("price: 12.5");
assert_eq!(p.amount, Some(12.5));
assert_eq!(p.currency, None);

// "free" → 0, no number → None
assert_eq!(Price::from_string("Free shipping").amount, Some(0.0));
assert_eq!(Price::from_string("call for price").amount, None);
```

You can pass hints or force the separators:

```rust
use price_parser::Price;

// Force the decimal separator so "140.000" means 140 (not 140000):
let p = Price::parse(Some("140.000"), None, Some("."), None);
assert_eq!(p.amount, Some(140.0));

// Strip a known thousands separator first:
let p = Price::parse(Some("1.000.000"), None, None, Some("."));
assert_eq!(p.amount, Some(1000000.0));
```

## How the amount is parsed

The decimal/thousands separator is guessed from the digits after the last separator: a group
of exactly 1, 2, or 4+ digits indicates a decimal separator; a group of 3 is treated as a
thousands separator. So `1,234` → `1234`, `12,34` → `12.34`, `12,345` → `12345`.

## Notes

- Amounts are returned as `f64`. The reference uses arbitrary-precision `Decimal`; values
  beyond `f64` range or precision (far larger or more precise than any real price) may
  differ.
- There is one intentional difference from the Python package: it has a niche special case
  where the euro sign appears *between digits* as a decimal separator (e.g. `"35€99"` →
  `35.99`). Replicating its exact regular expression requires conditional matching that the
  Rust regex engine does not support, so such inputs are parsed by the ordinary number
  logic instead. Currency detection and the standard decimal/thousands heuristic match the
  reference exactly.

## Contributors ✨

This project follows the [all-contributors](https://github.com/all-contributors/all-contributors) specification. Contributions of any kind are welcome — code, docs, bug reports, ideas, reviews! See the [emoji key](https://allcontributors.org/docs/en/emoji-key) for how each contribution is recognized, and open a PR or issue to get involved.

Thanks goes to these wonderful people:

<!-- ALL-CONTRIBUTORS-LIST:START - Do not remove or modify this section -->
<!-- prettier-ignore-start -->
<!-- markdownlint-disable -->
<table>
  <tbody>
    <tr>
      <td align="center" valign="top" width="14.28%"><a href="https://github.com/trananhtung"><img src="https://avatars.githubusercontent.com/u/30992229?v=4?s=100" width="100px;" alt="Tung Tran"/><br /><sub><b>Tung Tran</b></sub></a><br /><a href="https://github.com/trananhtung/./commits?author=trananhtung" title="Code">💻</a> <a href="#maintenance-trananhtung" title="Maintenance">🚧</a></td>
    </tr>
  </tbody>
</table>

<!-- markdownlint-restore -->
<!-- prettier-ignore-end -->

<!-- ALL-CONTRIBUTORS-LIST:END -->

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
