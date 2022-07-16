//! A parser for [hledger](https://hledger.org/) plaintext accounting files.

#![warn(missing_docs)]

use nom::bytes::complete::{take_till, take_till1};
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
    /// A potential comment after the description.
    pub comment: Option<String>,
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
    pub value: Option<ValueAndExchange>,
    /// A possible comment.
    pub comment: Option<String>,
}

impl Line {
    fn parse(i: &str) -> IResult<&str, Line> {
        let (i, account) = Line::parse_account(i)?;
        let (i, value) = opt(Line::parse_value)(i)?;
        let (i, comment) = opt(Line::parse_comment)(i)?;

        let line = Line {
            account,
            value,
            comment,
        };
        Ok((i, line))
    }

    fn parse_account(i: &str) -> IResult<&str, String> {
        let (i, account) = take_till1(|c| c == ' ' || c == '\n')(i)?;

        Ok((i, account.to_string()))
    }

    fn parse_value(i: &str) -> IResult<&str, ValueAndExchange> {
        let (i, _) = char(' ')(i)?;
        let (i, _) = space1(i)?;
        ValueAndExchange::parse(i)
    }

    fn parse_comment(i: &str) -> IResult<&str, String> {
        let (i, _) = space1(i)?;
        parse_comment(i)
    }
}

/// A [`Value`] potentially paired with an [`Exchange`].
pub struct ValueAndExchange {
    /// Known symbols: `=`
    pub symbol: Option<char>,
    /// The monetary value.
    pub value: Value,
    /// A possible exchange rate.
    pub exchange: Option<Exchange>,
}

impl ValueAndExchange {
    fn parse(i: &str) -> IResult<&str, ValueAndExchange> {
        let (i, symbol) = opt(char('='))(i)?;
        let (i, _) = space1(i)?;
        let (i, value) = Value::parse(i)?;
        let (i, exchange) = opt(ValueAndExchange::parse_exchange)(i)?;

        let vae = ValueAndExchange {
            symbol,
            value,
            exchange,
        };
        Ok((i, vae))
    }

    fn parse_exchange(i: &str) -> IResult<&str, Exchange> {
        let (i, _) = space1(i)?;
        Exchange::parse(i)
    }
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

impl Exchange {
    fn parse(i: &str) -> IResult<&str, Exchange> {
        todo!()
    }
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
    fn lines() {
        let line = "assets:cash:stash    200000 Y @@ 1927.20 C";
        let (rem, parsed) = Line::parse(line).unwrap();
        assert_eq!("", rem);
        // assert_eq!(200000, parsed.value.unwrap().value.value);
    }

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
