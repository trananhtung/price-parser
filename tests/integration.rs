//! Integration tests exercising the public API of `price-parser`.

use price_parser::Price;

fn amt(s: &str) -> Option<f64> {
    Price::from_string(s).amount
}
fn cur(s: &str) -> Option<String> {
    Price::from_string(s).currency
}

#[test]
fn realistic_prices() {
    let p = Price::from_string("$1,234.56");
    assert_eq!(p.amount, Some(1234.56));
    assert_eq!(p.currency.as_deref(), Some("$"));
    assert_eq!(p.amount_text.as_deref(), Some("1,234.56"));

    assert_eq!(amt("US$ 19.99"), Some(19.99));
    assert_eq!(cur("US$ 19.99").as_deref(), Some("US$"));
    assert_eq!(amt("1.234,56 €"), Some(1234.56));
    assert_eq!(cur("1.234,56 €").as_deref(), Some("€"));
}

#[test]
fn separator_heuristics() {
    assert_eq!(amt("1,234"), Some(1234.0));
    assert_eq!(amt("12,34"), Some(12.34));
    assert_eq!(amt("12,345"), Some(12345.0));
    assert_eq!(amt("1.000"), Some(1000.0));
    assert_eq!(amt("1 234.99"), Some(1234.99));
    assert_eq!(amt("3,0000"), Some(3.0));
    assert_eq!(amt("12.34.56"), None);
}

#[test]
fn dollar_code_priority() {
    assert_eq!(cur("NZD$123").as_deref(), Some("NZD"));
    assert_eq!(cur("SGD$1,000").as_deref(), Some("SGD"));
}

#[test]
fn special_cases() {
    assert_eq!(amt("free"), Some(0.0));
    assert_eq!(amt("FREE!"), Some(0.0));
    assert_eq!(amt("no price"), None);
    assert_eq!(amt("50%"), None);
    assert_eq!(amt("-5"), Some(5.0));
    assert_eq!(amt("$.99"), Some(0.99));
}
