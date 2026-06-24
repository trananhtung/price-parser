//! # price-parser — extract a price amount and currency from a string
//!
//! Given messy text such as `"$1,234.56"`, `"1.234,56 €"`, or `"Rs. 99"`, recover the
//! numeric amount and the currency symbol, guessing the decimal and thousands separators.
//! A faithful Rust port of the [`price-parser`](https://pypi.org/project/price-parser/)
//! Python package (Zyte).
//!
//! ```
//! use price_parser::Price;
//!
//! let p = Price::from_string("$1,234.56");
//! assert_eq!(p.amount, Some(1234.56));
//! assert_eq!(p.currency.as_deref(), Some("$"));
//!
//! // European decimal/thousands convention is detected.
//! let p = Price::from_string("1.234,56 EUR");
//! assert_eq!(p.amount, Some(1234.56));
//! assert_eq!(p.currency.as_deref(), Some("EUR"));
//! ```
//!
//! ## Limitation
//!
//! The Python package has a niche special case where the euro sign is used *between digits*
//! as a decimal separator (e.g. `"35€99"` → `35.99`); replicating its exact regex requires
//! conditional matching not available here, so such inputs are parsed by the ordinary
//! number logic instead. Every other behavior — currency detection and the
//! decimal/thousands heuristic — matches the reference.

#![forbid(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/price-parser/0.1.0")]
#![allow(clippy::unreadable_literal)]

use regex::Regex;
use std::sync::OnceLock;

mod currencies;
use currencies::{DOLLAR_CODES, OTHER_CURRENCY_SYMBOLS, SAFE_CURRENCY_SYMBOLS};

// Compile-test the README's examples as part of `cargo test`.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

/// A parsed price: a numeric `amount`, a `currency` symbol (as it appeared), and the raw
/// `amount_text` that was extracted.
#[derive(Debug, Clone, PartialEq)]
pub struct Price {
    /// The numeric price value, or `None` if no number was found.
    pub amount: Option<f64>,
    /// The currency symbol as it appeared in the text, or `None`.
    pub currency: Option<String>,
    /// The raw amount substring that was extracted, or `None`.
    pub amount_text: Option<String>,
}

impl Price {
    /// Parse a price from a string.
    ///
    /// ```
    /// # use price_parser::Price;
    /// assert_eq!(Price::from_string("Rs. 99").amount, Some(99.0));
    /// ```
    #[must_use]
    pub fn from_string(price: &str) -> Price {
        Price::parse(Some(price), None, None, None)
    }

    /// Parse a price with optional hints.
    ///
    /// * `currency_hint` — extra text that may contain the currency.
    /// * `decimal_separator` — force the decimal separator (`"."` or `","`) instead of
    ///   guessing.
    /// * `digit_group_separator` — a thousands separator to strip before parsing.
    #[must_use]
    pub fn parse(
        price: Option<&str>,
        currency_hint: Option<&str>,
        decimal_separator: Option<&str>,
        digit_group_separator: Option<&str>,
    ) -> Price {
        let currency = extract_currency_symbol(price, currency_hint).map(|c| c.trim().to_string());

        let stripped;
        let price = match (digit_group_separator, price) {
            (Some(sep), Some(p)) => {
                stripped = p.replace(sep, "");
                Some(stripped.as_str())
            }
            _ => price,
        };

        let amount_text = price.and_then(extract_price_text);
        let amount = amount_text
            .as_deref()
            .and_then(|t| parse_number(t, decimal_separator));

        Price {
            amount,
            currency,
            amount_text,
        }
    }
}

/// Build an alternation regex over `symbols` (each escaped), matching any of them.
fn or_regex(symbols: &[&str]) -> Regex {
    let pattern: String = symbols
        .iter()
        .map(|s| regex_escape(s))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&pattern).expect("currency alternation is valid")
}

fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if "\\.+*?()|[]{}^$#&-~".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn safe_currency_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| or_regex(SAFE_CURRENCY_SYMBOLS))
}

fn other_currency_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| or_regex(OTHER_CURRENCY_SYMBOLS))
}

fn extract_currency_symbol(price: Option<&str>, currency_hint: Option<&str>) -> Option<String> {
    // Method order mirrors the reference, including the `$`-code insertions at the front.
    let mut methods: Vec<(Method, Option<&str>)> = vec![
        (Method::Safe, price),
        (Method::Safe, currency_hint),
        (Method::Other, price),
        (Method::Other, currency_hint),
    ];
    if currency_hint.is_some_and(|h| h.contains('$')) {
        methods.insert(0, (Method::Dollar, currency_hint));
    }
    if price.is_some_and(|p| p.contains('$')) {
        methods.insert(0, (Method::Dollar, price));
    }

    for (method, value) in methods {
        let Some(value) = value else { continue };
        let found = match method {
            Method::Safe => safe_currency_re()
                .find(value)
                .map(|m| m.as_str().to_string()),
            Method::Other => other_currency_re()
                .find(value)
                .map(|m| m.as_str().to_string()),
            Method::Dollar => search_dollar_code(value),
        };
        if let Some(found) = found {
            return Some(found);
        }
    }
    None
}

#[derive(Clone, Copy)]
enum Method {
    Safe,
    Other,
    Dollar,
}

/// Hand-rolled equivalent of `_DOLLAR_REGEX`:
/// `\b(?:CODE)(?=\$?(?:[\W\d]|$))` — a dollar code at a word boundary, followed (after an
/// optional `$`) by a non-letter character or end of string. Returns the leftmost match.
fn search_dollar_code(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let n = bytes.len();
    let mut best: Option<(usize, &str)> = None;
    for &code in DOLLAR_CODES {
        let cb = code.as_bytes();
        let mut start = 0;
        while let Some(rel) = value[start..].find(code) {
            let pos = start + rel;
            // Word boundary before: previous byte is not a word char.
            let boundary = pos == 0 || !is_word_byte(bytes[pos - 1]);
            if boundary {
                // Lookahead `\$?(?:[\W\d]|$)`: since `$` is itself a non-word character, this
                // reduces to "the character after the code is not a letter/underscore, or
                // end of string" (the optional `$` adds nothing the first branch misses).
                let after = pos + cb.len();
                let look_ok = after >= n || !is_letter_or_underscore(bytes[after]);
                if look_ok && best.map_or(true, |(bp, _)| pos < bp) {
                    best = Some((pos, code));
                    break;
                }
            }
            start = pos + 1;
            if start >= n {
                break;
            }
        }
    }
    best.map(|(_, c)| c.to_string())
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_letter_or_underscore(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([.]?\d[\d\s.,']*)\s*?(?:[^%\d]|$)").expect("number regex"))
}

fn extract_price_text(price: &str) -> Option<String> {
    // Normalize all whitespace (including non-breaking) to single spaces.
    static WS: OnceLock<Regex> = OnceLock::new();
    let ws = WS.get_or_init(|| Regex::new(r"\s+").expect("ws regex"));
    let price = ws.replace_all(price, " ");

    if let Some(caps) = number_re().captures(&price) {
        let group1 = caps.get(1).map_or("", |m| m.as_str());
        let trimmed = group1.trim_end_matches([',', '.']).replace('\'', "");
        let result = if trimmed.matches('.').count() == 1 {
            trimmed.trim().to_string()
        } else {
            trimmed.trim_start_matches([',', '.']).trim().to_string()
        };
        return Some(result);
    }
    if price.to_lowercase().contains("free") {
        return Some("0".to_string());
    }
    None
}

fn decimal_sep_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\d*([.,€])(?:\d{1,2}?|\d{4}\d*?)$").expect("decimal sep regex"))
}

fn get_decimal_separator(price: &str) -> Option<String> {
    decimal_sep_re()
        .captures(price)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn parse_number(num: &str, decimal_separator: Option<&str>) -> Option<f64> {
    if num.is_empty() {
        return None;
    }
    let mut num = num.trim().replace(' ', "");
    let sep = decimal_separator
        .map(ToString::to_string)
        .or_else(|| get_decimal_separator(&num));

    num = match sep.as_deref() {
        None => num.replace(['.', ','], ""),
        Some(".") => num.replace(',', ""),
        Some(",") => num.replace('.', "").replace(',', "."),
        Some("€") => num.replace(['.', ','], "").replace('€', "."),
        // The reference only ever produces ".", ",", or "€"; any explicit override is used
        // as the decimal separator directly.
        Some(other) => num.replacen(other, ".", usize::MAX),
    };

    num.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(s: &str) -> Option<f64> {
        Price::from_string(s).amount
    }
    fn currency(s: &str) -> Option<String> {
        Price::from_string(s).currency
    }

    #[test]
    fn basic_amounts() {
        assert_eq!(amount("$1,234.56"), Some(1234.56));
        assert_eq!(amount("USD 1,000"), Some(1000.0));
        assert_eq!(amount("price: 12.5"), Some(12.5));
        assert_eq!(amount("-5"), Some(5.0));
        assert_eq!(amount("free"), Some(0.0));
        assert_eq!(amount("no price here"), None);
    }

    #[test]
    fn decimal_vs_thousands_heuristic() {
        assert_eq!(amount("1,234"), Some(1234.0));
        assert_eq!(amount("12,34"), Some(12.34));
        assert_eq!(amount("12,345"), Some(12345.0));
        assert_eq!(amount("1.000"), Some(1000.0));
        assert_eq!(amount("1.234,56"), Some(1234.56));
        assert_eq!(amount("1 234.99"), Some(1234.99));
        assert_eq!(amount("12.34.56"), None);
    }

    #[test]
    fn currency_detection() {
        assert_eq!(currency("$1,234.56").as_deref(), Some("$"));
        assert_eq!(currency("1.234,56 EUR").as_deref(), Some("EUR"));
        assert_eq!(currency("Rs. 99").as_deref(), Some("Rs"));
        assert_eq!(currency("19.90 dollars"), None);
        assert_eq!(currency("£1.5").as_deref(), Some("£"));
    }

    #[test]
    fn forced_separators() {
        assert_eq!(
            Price::parse(Some("140.000"), None, Some(","), None).amount,
            Some(140000.0)
        );
        assert_eq!(
            Price::parse(Some("140.000"), None, Some("."), None).amount,
            Some(140.0)
        );
        assert_eq!(
            Price::parse(Some("1.000.000"), None, None, Some(".")).amount,
            Some(1000000.0)
        );
    }
}
