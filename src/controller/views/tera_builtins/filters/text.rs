//! Text filters: `pluralize`, `to_sentence`, `ordinalize`.
//!
//! The wording lives in [`crate::controller::views::helpers`]; these are thin
//! Tera wrappers over it.

use tera::{Error, Kwargs, State, TeraResult, Value};

use crate::controller::views::helpers::{self, SentenceConnectors};

/// `{{ count | pluralize(word="person") }}` renders `"2 people"`.
///
/// With `word`, this is Rails' `pluralize(count, singular, plural:)`: the
/// count followed by `word`, or its plural (`plural=` overrides the
/// inflector). Without `word`, it keeps Tera's own `pluralize` behaviour and
/// returns only a suffix (`singular=""`, `plural="s"`), so existing templates
/// are unaffected.
///
/// # Errors
///
/// Returns an error if a keyword argument has the wrong type, or if, without
/// `word`, the value is not an integer.
// `Kwargs` by value is Tera's `Filter` signature, not a choice.
#[allow(clippy::needless_pass_by_value)]
pub fn pluralize(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let word: Option<&str> = kwargs.get("word")?;
    let plural: Option<&str> = kwargs.get("plural")?;
    if let Some(word) = word {
        let count = value
            .as_str()
            .map_or_else(|| value.to_string(), ToString::to_string);
        return Ok(Value::from(helpers::pluralize(count, word, plural)));
    }

    // Tera's built-in `pluralize`, which this filter replaces.
    let singular: &str = kwargs.get("singular")?.unwrap_or("");
    let plural = plural.unwrap_or("s");
    let Some(n) = value.as_i128() else {
        return Err(Error::message(format!(
            "pluralize filter requires an integer, got `{}`",
            value.name()
        )));
    };
    Ok(Value::from(if n == 1 || n == -1 {
        singular
    } else {
        plural
    }))
}

/// `{{ names | to_sentence }}` renders `"a, b, and c"`.
///
/// Takes Rails' `words_connector`, `two_words_connector` and
/// `last_word_connector` keyword arguments. Non-string items are written as
/// they would print.
///
/// # Errors
///
/// Returns an error if the value is not an array, or a connector is not a
/// string.
#[allow(clippy::needless_pass_by_value)]
pub fn to_sentence(value: &Value, kwargs: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let Some(items) = value.as_array() else {
        return Err(Error::message(format!(
            "to_sentence filter requires an array, got `{}`",
            value.name()
        )));
    };
    let words: Vec<String> = items
        .iter()
        .map(|item| {
            item.as_str()
                .map_or_else(|| item.to_string(), ToString::to_string)
        })
        .collect();
    let defaults = SentenceConnectors::default();
    let connectors = SentenceConnectors {
        words_connector: kwargs
            .get("words_connector")?
            .unwrap_or(defaults.words_connector),
        two_words_connector: kwargs
            .get("two_words_connector")?
            .unwrap_or(defaults.two_words_connector),
        last_word_connector: kwargs
            .get("last_word_connector")?
            .unwrap_or(defaults.last_word_connector),
    };
    Ok(Value::from(helpers::to_sentence_with(&words, &connectors)))
}

/// `{{ place | ordinalize }}` renders `"1st"`, `"22nd"`, `"113th"`.
///
/// # Errors
///
/// Never fails: a value that is not an integer (or an integer string) is
/// returned unchanged.
pub fn ordinalize(value: &Value, _: Kwargs, _: &State<'_>) -> TeraResult<Value> {
    let n = value
        .as_i64()
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()));
    Ok(n.map_or_else(|| value.clone(), |n| Value::from(helpers::ordinalize(n))))
}
