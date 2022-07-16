//! A parser for [hledger](https://hledger.org/) plaintext accounting files.

#![warn(missing_docs)]

use time::Date;

/// A block of text can either be a full [`Entry`], or just a comment.
pub enum EntryOrComment {
    /// A proper entry.
    Entry(Entry),
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
pub struct Value {
    /// The actual monetary value.
    pub value: f64,
    /// Some currency marker like `CAD` or `YEN`.
    pub currency: Option<String>,
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
