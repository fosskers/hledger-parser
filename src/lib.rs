//! A parser for [hledger](https://hledger.org/) plaintext accounting files.

#![warn(missing_docs)]

use nom::bytes::complete::take_till;
use nom::character::complete::{alpha1, char, digit1, space1};
use nom::combinator::{fail, map_res, opt};
use nom::number::complete::double;
use nom::IResult;
use time::{Date, Month};

/// A standalone block of text can represent a number of things.
pub enum Block {
    /// A proper entry.
    Entry(Entry),
    /// The price of some assets as of a certain date.
    Price(Price),
    /// Some comment line.
    Comment(String),
}

/// Represents the flow of some funds between one or more accounts.
pub struct Entry {
    /// The date of the transaction.
    pub date: Date,
    /// A short description appearing on the same line as the date.
    pub description: String,
    /// At least two [`Line`]s with any amount of comments peppered in between.
    pub lines: Vec<LineOrComment>,
}

/// A line of an [`Entry`] may be a comment or a proper [`Line`].
pub enum LineOrComment {
    /// A proper [`Line`] of an [`Entry`].
    Line(Line),
    /// A single comment line.
    Comment(String),
}

/// One element of a full transaction [`Entry`].
pub struct Line {
    /// Something like `expenses:food`.
    pub account: String,
    /// The monetary value.
    pub value: Value,
    /// A possible exchange rate.
    pub exchange: Option<Exchange>,
    /// A possible comment.
    pub comment: Option<String>,
}

/// A monetary value, hopefully paired with a currency marker.
#[derive(Debug)]
pub struct Value {
    /// The actual monetary value.
    pub value: f64,
    /// Some currency marker like `CAD` or `YEN`.
    pub currency: Option<String>,
}

impl Value {
    fn parse(i: &str) -> IResult<&str, Value> {
        let (i, value) = double(i)?;
        let (i, currency) = opt(Value::parse_currency)(i)?;

        let value = Value { value, currency };
        Ok((i, value))
    }

    fn parse_currency(i: &str) -> IResult<&str, String> {
        let (i, _) = space1(i)?;
        let (i, currency) = alpha1(i)?;

        Ok((i, currency.to_string()))
    }
}

/// An exchange rate that may be associated with a [`Line`].
pub enum Exchange {
    /// The cost of exchanging one unit, as in:
    ///
    /// > 11.23 CAD @ 1.21 USD
    PerUnit(Value),
    /// The cost of the entire exchange, as in:
    ///
    /// > 200000 YEN @@ 1927.20 CAD
    Total(Value),
}

/// Some updated price of a moving asset.
///
/// > P 2022-07-12 TSLA 699.21 U
#[derive(Debug)]
pub struct Price {
    /// The date the new price was recorded.
    pub date: Date,
    /// The asset's label.
    pub asset: String,
    /// The new value.
    pub value: Value,
    /// A possible comment.
    pub comment: Option<String>,
}

impl Price {
    /// Construct a new `Price`.
    pub fn new<S>(date: Date, asset: S, value: Value, comment: Option<String>) -> Self
    where
        S: Into<String>,
    {
        Self {
            date,
            asset: asset.into(),
            value,
            comment,
        }
    }

    fn parse(i: &str) -> IResult<&str, Price> {
        let (i, _) = char('P')(i)?;
        let (i, _) = space1(i)?;
        let (i, date) = parse_date(i)?;
        let (i, _) = space1(i)?;
        let (i, asset) = alpha1(i)?;
        let (i, _) = space1(i)?;
        let (i, value) = Value::parse(i)?;
        let (i, comment) = opt(Price::parse_comment)(i)?;

        let price = Price::new(date, asset, value, comment);
        Ok((i, price))
    }

    fn parse_comment(i: &str) -> IResult<&str, String> {
        let (i, _) = space1(i)?;
        parse_comment(i)
    }
}

fn parse_date(i: &str) -> IResult<&str, Date> {
    let (i, year) = map_res(digit1, str::parse)(i)?;
    let (i, _) = char('-')(i)?;
    let (i, month) = parse_month(i)?;
    let (i, _) = char('-')(i)?;
    let (i, day) = map_res(digit1, str::parse)(i)?;

    match Date::from_calendar_date(year, month, day) {
        Ok(date) => Ok((i, date)),
        Err(_) => fail(i),
    }
}

fn parse_month(i: &str) -> IResult<&str, Month> {
    let (i, m) = map_res(digit1, str::parse)(i)?;

    match m {
        1 => Ok((i, Month::January)),
        2 => Ok((i, Month::February)),
        3 => Ok((i, Month::March)),
        4 => Ok((i, Month::April)),
        5 => Ok((i, Month::May)),
        6 => Ok((i, Month::June)),
        7 => Ok((i, Month::July)),
        8 => Ok((i, Month::August)),
        9 => Ok((i, Month::September)),
        10 => Ok((i, Month::October)),
        11 => Ok((i, Month::November)),
        12 => Ok((i, Month::December)),
        _ => fail(i),
    }
}

fn parse_comment(i: &str) -> IResult<&str, String> {
    let (i, _) = char(';')(i)?;
    let (i, comment) = take_till(|c| c == '\n')(i)?;

    Ok((i, comment.to_string()))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn dates() {
        let date = "2022-07-16";
        assert!(parse_date(date).is_ok());
    }

    #[test]
    fn prices() {
        let price = "P 2022-07-12 TSLA 699.21 U ; great buy?";
        let (_, parsed) = Price::parse(price).unwrap();
        assert_eq!(parsed.asset, "TSLA");
        assert_eq!(parsed.value.value, 699.21);
    }
}
