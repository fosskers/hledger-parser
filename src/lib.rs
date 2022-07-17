//! A parser for [hledger](https://hledger.org/) plaintext accounting files.

#![warn(missing_docs)]

use nom::bytes::complete::{take_till, take_till1};
use nom::character::complete::{alpha1, char, digit1, i64, space1, u64};
use nom::combinator::{fail, map_res, opt};
use nom::multi::many0_count;
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
    pub value: Number,
    /// Some currency marker like `CAD` or `YEN`.
    pub currency: Option<String>,
}

impl Value {
    fn parse(i: &str) -> IResult<&str, Value> {
        let (i, value) = Number::parse(i)?;
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

/// Either a plain integer or floating-point value.
///
/// Some currencies are not divisible. Even if they are, some entries made by
/// the user do not include decimal values. If they don't, then we wouldn't want
/// to render them with extra zeroes (etc.) during pretty-printing if they
/// didn't start with any.
#[derive(Debug)]
pub enum Number {
    /// An indivisible positive or negative integer.
    Int(i64),
    /// Any other number with decimal values.
    ///
    /// The three inner values are:
    /// - Signed value left of the decimal point.
    /// - The number of zeroes following the decimal point.
    /// - The final digits, as-is, if there are any.
    Float(i64, usize, Option<u64>),
}

impl Number {
    fn parse(i: &str) -> IResult<&str, Number> {
        let (i, int) = i64(i)?;
        match opt(Number::parse_float_parts)(i)? {
            (i, None) => Ok((i, Number::Int(int))),
            (i, Some((zeroes, last))) => Ok((i, Number::Float(int, zeroes, last))),
        }
    }

    fn parse_float_parts(i: &str) -> IResult<&str, (usize, Option<u64>)> {
        let (i, _) = char('.')(i)?;
        let (i, zeroes) = many0_count(char('0'))(i)?;
        let (i, last) = opt(u64)(i)?;

        Ok((i, (zeroes, last)))
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(l), Number::Int(r)) => l == r,
            (Number::Int(l), Number::Float(r, _, None)) => l == r,
            (Number::Int(_), Number::Float(_, _, Some(_))) => false,
            (Number::Float(l, _, None), Number::Int(r)) => l == r,
            (Number::Float(_, _, Some(_)), Number::Int(_)) => false,
            (Number::Float(l, _, None), Number::Float(r, _, None)) => l == r,
            (Number::Float(_, _, Some(_)), Number::Float(_, _, None)) => false,
            (Number::Float(_, _, None), Number::Float(_, _, Some(_))) => false,
            (Number::Float(l, a, Some(x)), Number::Float(r, b, Some(y))) => {
                l == r && a == b && x == y
            }
        }
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
    fn numbers() {
        let nums = [
            (Number::Int(600), "600"),
            (Number::Float(600, 3, None), "600.000"),
            (Number::Float(600, 3, Some(123)), "600.000123"),
        ];

        nums.into_iter().for_each(|(exp, s)| {
            let (rem, parsed) = Number::parse(s).unwrap();
            assert_eq!("", rem);
            assert_eq!(exp, parsed);
        });

        assert_eq!(Number::Int(600), Number::Float(600, 1000, None));
    }

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
        let (rem, parsed) = Price::parse(price).unwrap();
        assert_eq!("", rem);
        assert_eq!(parsed.asset, "TSLA");
        assert_eq!(parsed.value.value, Number::Float(699, 0, Some(21)));
    }
}
