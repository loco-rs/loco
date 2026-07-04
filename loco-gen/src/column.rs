//! A single, compiler-checked source of truth for column type information.
//!
//! This module replaces the four parallel tables the old field-type-mapping
//! JSON file used to carry (`rust`, `schema`, `col_type`, `arity`) with one
//! Rust enum (`ScalarType`) and four small, exhaustively-matched derivation
//! functions on [`Column`]. Every derivation is a `match` over the type
//! model, so adding a new `ScalarType` variant without updating a derivation
//! is a compile error, not a silently-stale JSON row.
//!
//! It is the single source of truth for column type information used by the
//! migration/model/scaffold generators.

use cruet::Inflector;
use heck::ToUpperCamelCase;

use crate::{Error, Result};

/// A scalar (non-array, non-reference) column type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarType {
    String,
    Text,
    Uuid,
    SmallInt,
    Int,
    BigInt,
    SmallUnsigned,
    Unsigned,
    BigUnsigned,
    Float,
    Double,
    Decimal,
    DecimalLen { precision: u32, scale: u32 },
    Money,
    Bool,
    Date,
    Time,
    DateTime,
    DateTimeTz,
    Json,
    Jsonb,
    Blob,
    VarBinary { len: u32 },
    BinaryLen { len: u32 },
    Enum { values: Vec<String> },
}

/// The shape of a column: a plain scalar, a Postgres/`SeaORM` array of a
/// scalar, or a belongs-to reference (always a 64-bit foreign key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnKind {
    Scalar(ScalarType),
    Array(ScalarType),
    Reference {
        target: String,
        fk_field: Option<String>,
    },
}

/// A fully-parsed column: its name, shape, and nullability/uniqueness flags.
///
/// Invariant upheld by [`parse_column`]: `unique` is only ever `true`
/// together with `nullable == false` (the DSL's `^` suffix means "unique
/// AND not null"). Combinations that have no matching `ColType` variant
/// (unique `bool`, unique `tstz`) are rejected at parse time so the
/// derivation methods below never need to invent one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
    pub nullable: bool,
    pub unique: bool,
}

/// The web form control a column should be edited with in a generated
/// (HTML/HTMX) scaffold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormKind {
    Text,
    Textarea,
    Number,
    Checkbox,
    DateTime,
    Select(Vec<String>),
}

/// Three-way flag used to pick the `ColType::{Stem}`/`{Stem}Null`/`{Stem}Uniq`
/// suffix. `unique` implies `!nullable` (enforced by `parse_column`); if that
/// invariant is ever violated, `Null` wins so we still emit a valid,
/// non-panicking `ColType` name rather than a made-up `*Uniq` variant that
/// doesn't exist for every stem.
enum Flag {
    Null,
    Uniq,
    Required,
}

impl Flag {
    const fn of(nullable: bool, unique: bool) -> Self {
        if nullable {
            Self::Null
        } else if unique {
            Self::Uniq
        } else {
            Self::Required
        }
    }

    fn suffixed(&self, stem: &str) -> String {
        match self {
            Self::Null => format!("{stem}Null"),
            Self::Uniq => format!("{stem}Uniq"),
            Self::Required => stem.to_string(),
        }
    }
}

/// Parse one `field:spec` column DSL string (the part after the field name)
/// into a [`Column`].
///
/// Grammar (orthogonal: suffixes are flags, not combinatorial rows):
/// * trailing `!`  => required (`NOT NULL`)
/// * trailing `^`  => unique + required
/// * no suffix     => nullable (bare is nullable, matching current semantics)
/// * `references` / `references?` / `references:custom_fk` /
///   `references?:custom_fk` => a 64-bit foreign key (`references` is
///   `NOT NULL`, `references?` is nullable -- the opposite bare-is-nullable
///   convention on purpose, matching the retired field-type parser this
///   replaced)
/// * `enum:a,b,c` (+ `!`/`^`)          => `ScalarType::Enum`
/// * `decimal_len:P:S` (+ `!`/`^`)     => `ScalarType::DecimalLen`
/// * `var_binary:N` (+ `!`/`^`)        => `ScalarType::VarBinary`
/// * `binary_len:N` (+ `!`/`^`)        => `ScalarType::BinaryLen`
/// * `array:inner` (+ `!`/`^`)         => `ColumnKind::Array` (inner is one
///   of `string`/`int`/`big_int`/`float`/`double`/`bool`)
/// * otherwise a bare scalar base name (see `scalar_from_base_name`)
///
/// # Errors
/// Returns `Error::Message` for: unknown base names, wrong arity/unparsable
/// parameters for `decimal_len`/`var_binary`/`binary_len`, an empty `enum`
/// value list, an unsupported `array` inner type, or a unique/nullable
/// combination that has no matching `ColType` (`bool^`, `tstz^`).
pub fn parse_column(name: &str, spec: &str) -> Result<Column> {
    if spec == "references"
        || spec == "references?"
        || spec.starts_with("references:")
        || spec.starts_with("references?:")
    {
        let nullable = spec.starts_with("references?");
        let fk_field = spec.split_once(':').and_then(|(_, field)| {
            if field.is_empty() {
                None
            } else {
                Some(field.to_string())
            }
        });
        return Ok(Column {
            name: name.to_string(),
            kind: ColumnKind::Reference {
                target: name.to_string(),
                fk_field,
            },
            nullable,
            unique: false,
        });
    }

    let (base, nullable, unique) = if let Some(stripped) = spec.strip_suffix('^') {
        (stripped, false, true)
    } else if let Some(stripped) = spec.strip_suffix('!') {
        (stripped, false, false)
    } else {
        (spec, true, false)
    };

    let parts: Vec<&str> = base.split(':').collect();
    let kind = match parts.as_slice() {
        ["array", rest @ ..] => {
            let [inner_name] = rest else {
                return Err(Error::Message(format!(
                    "type `array` requires exactly 1 parameter (`array:inner`), got {} (`{base}`)",
                    rest.len()
                )));
            };
            ColumnKind::Array(array_inner_from_name(inner_name)?)
        }
        ["enum", rest @ ..] => {
            let [values] = rest else {
                return Err(Error::Message(format!(
                    "type `enum` requires exactly 1 parameter (`enum:a,b,c`), got {} (`{base}`)",
                    rest.len()
                )));
            };
            let values: Vec<String> = values.split(',').map(str::trim).map(String::from).collect();
            if values.iter().any(String::is_empty) {
                return Err(Error::Message(format!(
                    "type `enum` requires at least one non-empty, comma-separated value \
                     (`enum:a,b,c`), got `{base}`"
                )));
            }
            ColumnKind::Scalar(ScalarType::Enum { values })
        }
        ["decimal_len", rest @ ..] => {
            let [p, s] = rest else {
                return Err(Error::Message(format!(
                    "type `decimal_len` requires exactly 2 parameters (`decimal_len:precision:scale`), \
                     got {} (`{base}`)",
                    rest.len()
                )));
            };
            let precision = p.parse::<u32>().map_err(|_| {
                Error::Message(format!("decimal_len: precision `{p}` is not a valid u32"))
            })?;
            let scale = s.parse::<u32>().map_err(|_| {
                Error::Message(format!("decimal_len: scale `{s}` is not a valid u32"))
            })?;
            ColumnKind::Scalar(ScalarType::DecimalLen { precision, scale })
        }
        ["var_binary", rest @ ..] => {
            let [n] = rest else {
                return Err(Error::Message(format!(
                    "type `var_binary` requires exactly 1 parameter (`var_binary:len`), got {} \
                     (`{base}`)",
                    rest.len()
                )));
            };
            let len = n
                .parse::<u32>()
                .map_err(|_| Error::Message(format!("var_binary: len `{n}` is not a valid u32")))?;
            ColumnKind::Scalar(ScalarType::VarBinary { len })
        }
        ["binary_len", rest @ ..] => {
            let [n] = rest else {
                return Err(Error::Message(format!(
                    "type `binary_len` requires exactly 1 parameter (`binary_len:len`), got {} \
                     (`{base}`)",
                    rest.len()
                )));
            };
            let len = n
                .parse::<u32>()
                .map_err(|_| Error::Message(format!("binary_len: len `{n}` is not a valid u32")))?;
            ColumnKind::Scalar(ScalarType::BinaryLen { len })
        }
        [basename] => ColumnKind::Scalar(scalar_from_base_name(basename)?),
        other => {
            return Err(Error::Message(format!(
                "unknown column type: `{}`",
                other.join(":")
            )));
        }
    };

    // No `ColType::BooleanUniq` / `ColType::TimestampWithTimeZoneUniq` exist
    // (checked against `schema.rs`), so reject the combination up front
    // instead of emitting a `col_type()` string for a variant that isn't
    // there.
    if unique {
        match &kind {
            ColumnKind::Scalar(ScalarType::Bool) => {
                return Err(Error::Message(
                    "type `bool` cannot be unique (`^`): no unique boolean column type exists"
                        .to_string(),
                ));
            }
            ColumnKind::Scalar(ScalarType::DateTimeTz) => {
                return Err(Error::Message(
                    "type `tstz` cannot be unique (`^`): no unique timestamptz column type exists"
                        .to_string(),
                ));
            }
            _ => {}
        }
    }

    Ok(Column {
        name: name.to_string(),
        kind,
        nullable,
        unique,
    })
}

/// Maps a bare scalar DSL base name to its `ScalarType`.
///
/// Preserves the retired field-type-mapping JSON's semantics for every name
/// **except** the deliberate 1.0 fix: `int` now means a real 32-bit
/// `Integer` (was `BigInteger`/i64 previously); use `big_int` for 64-bit.
/// `unsigned` is kept as an alias of `big_unsigned` (both mapped to
/// `BigUnsigned`/i64 previously) -- see the module-level note in the report
/// for why the dedicated `ScalarType::Unsigned` variant is not reachable
/// from this name.
///
/// # Errors
/// Returns `Error::Message` when `name` is not a recognized base type.
fn scalar_from_base_name(name: &str) -> Result<ScalarType> {
    match name {
        "string" => Ok(ScalarType::String),
        "text" => Ok(ScalarType::Text),
        "uuid" => Ok(ScalarType::Uuid),
        "bool" => Ok(ScalarType::Bool),
        "date" => Ok(ScalarType::Date),
        "time" => Ok(ScalarType::Time),
        "date_time" => Ok(ScalarType::DateTime),
        "tstz" => Ok(ScalarType::DateTimeTz),
        "json" => Ok(ScalarType::Json),
        "jsonb" => Ok(ScalarType::Jsonb),
        "blob" => Ok(ScalarType::Blob),
        "money" => Ok(ScalarType::Money),
        "decimal" => Ok(ScalarType::Decimal),
        "float" => Ok(ScalarType::Float),
        "double" => Ok(ScalarType::Double),
        "small_int" => Ok(ScalarType::SmallInt),
        "small_unsigned" => Ok(ScalarType::SmallUnsigned),
        "unsigned" | "big_unsigned" => Ok(ScalarType::BigUnsigned),
        "int" => Ok(ScalarType::Int),
        "big_int" => Ok(ScalarType::BigInt),
        other => Err(Error::Message(format!(
            "type: `{other}` not found. try any of: string,text,uuid,bool,date,time,date_time,\
             tstz,json,jsonb,blob,money,decimal,float,double,small_int,small_unsigned,unsigned,\
             big_unsigned,int,big_int,enum:..,decimal_len:..,var_binary:..,binary_len:..,array:..,\
             references"
        ))),
    }
}

/// The restricted subset of scalar types an `array:inner` column may hold,
/// matching the inner types the retired field-type-mapping JSON's `array`
/// rust-type map already enumerated
/// (`string`/`int`/`big_int`/`float`/`double`/`bool`).
///
/// # Errors
/// Returns `Error::Message` when `name` is not one of the six supported
/// array element types.
fn array_inner_from_name(name: &str) -> Result<ScalarType> {
    match name {
        "string" => Ok(ScalarType::String),
        "int" => Ok(ScalarType::Int),
        "big_int" => Ok(ScalarType::BigInt),
        "float" => Ok(ScalarType::Float),
        "double" => Ok(ScalarType::Double),
        "bool" => Ok(ScalarType::Bool),
        other => Err(Error::Message(format!(
            "type `array:{other}` is not supported. array elements must be one of: string,int,\
             big_int,float,double,bool"
        ))),
    }
}

/// Fields that are skipped when building columns/references from raw
/// scaffold input: these are automatically generated by Loco (created_at /
/// updated_at, plus the `create_at`/`update_at` typo variants that were
/// historically also special-cased). Kept in sync with, but intentionally
/// duplicated from, `model::IGNORE_FIELDS` -- `column` is the standalone,
/// bottom-of-the-stack module and must not depend upward on `model`.
const IGNORE_FIELDS: &[&str] = &["created_at", "updated_at", "create_at", "update_at"];

/// Parse raw `(field_name, spec)` scaffold fields into `Column`s, skipping
/// the auto-managed timestamp fields (see [`IGNORE_FIELDS`]).
///
/// # Errors
/// Returns the first `Error` produced by [`parse_column`] for any
/// non-skipped field.
pub fn columns_from_fields(fields: &[(String, String)]) -> Result<Vec<Column>> {
    fields
        .iter()
        .filter_map(|(name, spec)| {
            if IGNORE_FIELDS.contains(&name.as_str()) {
                tracing::warn!(
                    field = name,
                    "note that a redundant field was specified, it is already generated \
                     automatically"
                );
                None
            } else {
                Some(parse_column(name, spec))
            }
        })
        .collect()
}

impl Column {
    /// The `ColType::…` expression a migration emits for this column, e.g.
    /// `"StringNull"`, `"DecimalLen(10, 2)"`, `"array(ArrayColType::BigInt)"`.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn col_type(&self) -> String {
        let flag = Flag::of(self.nullable, self.unique);
        match &self.kind {
            ColumnKind::Reference { .. } => "BigInteger".to_string(),
            ColumnKind::Array(inner) => {
                let inner_name = array_col_type_name(inner);
                match flag {
                    Flag::Null => format!("array_null(ArrayColType::{inner_name})"),
                    Flag::Uniq => format!("array_uniq(ArrayColType::{inner_name})"),
                    Flag::Required => format!("array(ArrayColType::{inner_name})"),
                }
            }
            ColumnKind::Scalar(scalar) => match scalar {
                ScalarType::String | ScalarType::Enum { .. } => flag.suffixed("String"),
                ScalarType::Text => flag.suffixed("Text"),
                ScalarType::Uuid => flag.suffixed("Uuid"),
                ScalarType::SmallInt => flag.suffixed("SmallInteger"),
                ScalarType::Int => flag.suffixed("Integer"),
                ScalarType::BigInt => flag.suffixed("BigInteger"),
                ScalarType::SmallUnsigned => flag.suffixed("SmallUnsigned"),
                ScalarType::Unsigned => flag.suffixed("Unsigned"),
                ScalarType::BigUnsigned => flag.suffixed("BigUnsigned"),
                ScalarType::Float => flag.suffixed("Float"),
                ScalarType::Double => flag.suffixed("Double"),
                ScalarType::Decimal => flag.suffixed("Decimal"),
                ScalarType::DecimalLen { precision, scale } => {
                    let stem = flag.suffixed("DecimalLen");
                    format!("{stem}({precision}, {scale})")
                }
                ScalarType::Money => flag.suffixed("Money"),
                ScalarType::Bool => flag.suffixed("Boolean"),
                ScalarType::Date => flag.suffixed("Date"),
                ScalarType::Time => flag.suffixed("Time"),
                ScalarType::DateTime => flag.suffixed("DateTime"),
                ScalarType::DateTimeTz => flag.suffixed("TimestampWithTimeZone"),
                ScalarType::Json => flag.suffixed("Json"),
                ScalarType::Jsonb => flag.suffixed("JsonBinary"),
                ScalarType::Blob => flag.suffixed("Blob"),
                ScalarType::VarBinary { len } => {
                    let stem = flag.suffixed("VarBinary");
                    format!("{stem}({len})")
                }
                ScalarType::BinaryLen { len } => {
                    let stem = flag.suffixed("BinaryLen");
                    format!("{stem}({len})")
                }
            },
        }
    }

    /// The DTO field Rust type, e.g. `"i32"`, `"Option<String>"`,
    /// `"Vec<i64>"`.
    #[must_use]
    pub fn dto_rust_type(&self) -> String {
        let base = match &self.kind {
            ColumnKind::Reference { .. } => "i64".to_string(),
            ColumnKind::Array(inner) => format!("Vec<{}>", scalar_rust_type(inner, &self.name)),
            ColumnKind::Scalar(scalar) => scalar_rust_type(scalar, &self.name),
        };
        if self.nullable {
            format!("Option<{base}>")
        } else {
            base
        }
    }

    /// The `#[ts(type = "…")]` override for this column, or `None` when
    /// ts-rs's native mapping is already correct.
    #[must_use]
    pub fn ts_type(&self) -> Option<String> {
        let base = match &self.kind {
            ColumnKind::Reference { .. } => Some("number"),
            ColumnKind::Array(_) => None,
            ColumnKind::Scalar(scalar) => scalar_ts_override(scalar),
        };
        match base {
            Some(t) if self.nullable => Some(format!("{t} | null")),
            Some(t) => Some(t.to_string()),
            None => None,
        }
    }

    /// The web form input control for this column.
    #[must_use]
    pub fn form_input(&self) -> FormKind {
        match &self.kind {
            ColumnKind::Reference { .. } => FormKind::Number,
            ColumnKind::Array(_) => FormKind::Textarea,
            ColumnKind::Scalar(scalar) => scalar_form_input(scalar),
        }
    }
}

/// `ArrayColType` variant name (matches `schema.rs`'s `ArrayColType`
/// exactly). Only ever called with the six scalar types `array_inner_from_name`
/// accepts; the fallback arm exists solely so this stays a total function
/// (no panic) even if that invariant were ever broken.
fn array_col_type_name(scalar: &ScalarType) -> &'static str {
    match scalar {
        ScalarType::Int => "Int",
        ScalarType::BigInt => "BigInt",
        ScalarType::Float => "Float",
        ScalarType::Double => "Double",
        ScalarType::Bool => "Bool",
        _ => "String",
    }
}

/// The unwrapped (non-`Option`) DTO Rust type for a scalar. `column_name` is
/// only used for `Enum`, whose Rust type is the PascalCase singular of the
/// column name (e.g. column `status` => type `Status`).
fn scalar_rust_type(scalar: &ScalarType, column_name: &str) -> String {
    match scalar {
        ScalarType::String | ScalarType::Text => "String".to_string(),
        ScalarType::Uuid => "Uuid".to_string(),
        ScalarType::SmallInt | ScalarType::SmallUnsigned => "i16".to_string(),
        ScalarType::Int => "i32".to_string(),
        ScalarType::BigInt | ScalarType::BigUnsigned => "i64".to_string(),
        ScalarType::Unsigned => "u32".to_string(),
        ScalarType::Float => "f32".to_string(),
        ScalarType::Double => "f64".to_string(),
        ScalarType::Decimal | ScalarType::DecimalLen { .. } | ScalarType::Money => {
            "Decimal".to_string()
        }
        ScalarType::Bool => "bool".to_string(),
        ScalarType::Date => "Date".to_string(),
        ScalarType::Time => "Time".to_string(),
        ScalarType::DateTime => "DateTime".to_string(),
        ScalarType::DateTimeTz => "DateTimeWithTimeZone".to_string(),
        ScalarType::Json | ScalarType::Jsonb => "serde_json::Value".to_string(),
        ScalarType::Blob | ScalarType::VarBinary { .. } | ScalarType::BinaryLen { .. } => {
            "Vec<u8>".to_string()
        }
        ScalarType::Enum { .. } => column_name.to_singular().to_upper_camel_case(),
    }
}

/// The `#[ts(type = "…")]` override for a bare scalar, ignoring nullability
/// (the caller appends `" | null"` when the column is nullable).
const fn scalar_ts_override(scalar: &ScalarType) -> Option<&'static str> {
    match scalar {
        ScalarType::SmallInt
        | ScalarType::Int
        | ScalarType::BigInt
        | ScalarType::SmallUnsigned
        | ScalarType::Unsigned
        | ScalarType::BigUnsigned => Some("number"),
        ScalarType::Decimal | ScalarType::DecimalLen { .. } | ScalarType::Money => Some("string"),
        ScalarType::Date | ScalarType::Time | ScalarType::DateTime | ScalarType::DateTimeTz => {
            Some("string")
        }
        ScalarType::Uuid => Some("string"),
        ScalarType::Json | ScalarType::Jsonb => Some("unknown"),
        ScalarType::String
        | ScalarType::Text
        | ScalarType::Bool
        | ScalarType::Float
        | ScalarType::Double
        | ScalarType::Enum { .. }
        | ScalarType::Blob
        | ScalarType::VarBinary { .. }
        | ScalarType::BinaryLen { .. } => None,
    }
}

/// The web form input control for a bare scalar.
fn scalar_form_input(scalar: &ScalarType) -> FormKind {
    match scalar {
        ScalarType::String | ScalarType::Uuid => FormKind::Text,
        ScalarType::Text
        | ScalarType::Json
        | ScalarType::Jsonb
        | ScalarType::Blob
        | ScalarType::VarBinary { .. }
        | ScalarType::BinaryLen { .. } => FormKind::Textarea,
        ScalarType::SmallInt
        | ScalarType::Int
        | ScalarType::BigInt
        | ScalarType::SmallUnsigned
        | ScalarType::Unsigned
        | ScalarType::BigUnsigned
        | ScalarType::Float
        | ScalarType::Double
        | ScalarType::Decimal
        | ScalarType::DecimalLen { .. }
        | ScalarType::Money => FormKind::Number,
        ScalarType::Bool => FormKind::Checkbox,
        ScalarType::Date | ScalarType::Time | ScalarType::DateTime | ScalarType::DateTimeTz => {
            FormKind::DateTime
        }
        ScalarType::Enum { values } => FormKind::Select(values.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, spec: &str) -> Column {
        parse_column(name, spec).unwrap_or_else(|e| panic!("failed to parse `{name}:{spec}`: {e}"))
    }

    // ---- parser: suffix flags -------------------------------------------

    #[test]
    fn bare_is_nullable() {
        let c = col("title", "string");
        assert!(c.nullable);
        assert!(!c.unique);
        assert_eq!(c.kind, ColumnKind::Scalar(ScalarType::String));
    }

    #[test]
    fn bang_is_required() {
        let c = col("title", "string!");
        assert!(!c.nullable);
        assert!(!c.unique);
        assert_eq!(c.kind, ColumnKind::Scalar(ScalarType::String));
    }

    #[test]
    fn caret_is_unique_and_required() {
        let c = col("title", "string^");
        assert!(!c.nullable);
        assert!(c.unique);
        assert_eq!(c.kind, ColumnKind::Scalar(ScalarType::String));
    }

    // ---- parser: references ----------------------------------------------

    #[test]
    fn references_is_not_null_by_default() {
        let c = col("user", "references");
        assert!(!c.nullable);
        assert!(!c.unique);
        assert_eq!(
            c.kind,
            ColumnKind::Reference {
                target: "user".to_string(),
                fk_field: None
            }
        );
    }

    #[test]
    fn references_question_mark_is_nullable() {
        let c = col("user", "references?");
        assert!(c.nullable);
        assert_eq!(
            c.kind,
            ColumnKind::Reference {
                target: "user".to_string(),
                fk_field: None
            }
        );
    }

    #[test]
    fn references_with_custom_fk_field() {
        let c = col("user", "references:admin_id");
        assert!(!c.nullable);
        assert_eq!(
            c.kind,
            ColumnKind::Reference {
                target: "user".to_string(),
                fk_field: Some("admin_id".to_string())
            }
        );
    }

    #[test]
    fn references_nullable_with_custom_fk_field() {
        let c = col("user", "references?:admin_id");
        assert!(c.nullable);
        assert_eq!(
            c.kind,
            ColumnKind::Reference {
                target: "user".to_string(),
                fk_field: Some("admin_id".to_string())
            }
        );
    }

    #[test]
    fn references_always_reduces_to_big_integer_col_type() {
        assert_eq!(col("user", "references").col_type(), "BigInteger");
        assert_eq!(col("user", "references?").col_type(), "BigInteger");
    }

    // ---- parser: enum ------------------------------------------------------

    #[test]
    fn enum_parses_values() {
        let c = col("status", "enum:draft,published");
        assert_eq!(
            c.kind,
            ColumnKind::Scalar(ScalarType::Enum {
                values: vec!["draft".to_string(), "published".to_string()]
            })
        );
        assert!(c.nullable);
    }

    #[test]
    fn enum_required_parses() {
        let c = col("status", "enum:draft,published!");
        assert!(!c.nullable);
        assert_eq!(
            c.kind,
            ColumnKind::Scalar(ScalarType::Enum {
                values: vec!["draft".to_string(), "published".to_string()]
            })
        );
    }

    // ---- parser: decimal_len / var_binary / binary_len / array ------------

    #[test]
    fn decimal_len_parses_precision_and_scale() {
        let c = col("price", "decimal_len:10:2");
        assert_eq!(
            c.kind,
            ColumnKind::Scalar(ScalarType::DecimalLen {
                precision: 10,
                scale: 2
            })
        );
    }

    #[test]
    fn var_binary_parses_len() {
        let c = col("data", "var_binary:16");
        assert_eq!(
            c.kind,
            ColumnKind::Scalar(ScalarType::VarBinary { len: 16 })
        );
    }

    #[test]
    fn binary_len_parses_len() {
        let c = col("data", "binary_len:16");
        assert_eq!(
            c.kind,
            ColumnKind::Scalar(ScalarType::BinaryLen { len: 16 })
        );
    }

    #[test]
    fn array_parses_inner_type() {
        let c = col("tags", "array:string");
        assert_eq!(c.kind, ColumnKind::Array(ScalarType::String));
    }

    // ---- parser: error cases -----------------------------------------------

    #[test]
    fn bool_unique_is_rejected() {
        assert!(parse_column("active", "bool^").is_err());
    }

    #[test]
    fn tstz_unique_is_rejected() {
        assert!(parse_column("happened_at", "tstz^").is_err());
    }

    #[test]
    fn enum_empty_value_list_is_rejected() {
        assert!(parse_column("status", "enum:").is_err());
    }

    #[test]
    fn decimal_len_non_numeric_is_rejected() {
        assert!(parse_column("price", "decimal_len:abc").is_err());
    }

    #[test]
    fn decimal_len_wrong_arity_is_rejected() {
        assert!(parse_column("price", "decimal_len:1").is_err());
    }

    #[test]
    fn unknown_type_is_rejected() {
        assert!(parse_column("thing", "not_a_real_type").is_err());
    }

    #[test]
    fn array_unsupported_inner_is_rejected() {
        assert!(parse_column("things", "array:uuid").is_err());
    }

    // ---- derivations: representative table test ---------------------------

    /// One row per `ScalarType`, exercising every derivation. This is the
    /// coverage that replaces the old snapshot-only guarantee: it directly
    /// asserts the exact `ColType::…` string, DTO Rust type, ts-rs override
    /// and form control for both a required and a nullable (and, where a
    /// `ColType::*Uniq` exists, a unique) instance of every type.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn derivations_table() {
        struct Case {
            name: &'static str,
            spec: &'static str,
            col_type: &'static str,
            dto_rust_type: &'static str,
            ts_type: Option<&'static str>,
            form: FormKind,
        }

        let cases = vec![
            // string
            Case {
                name: "title",
                spec: "string!",
                col_type: "String",
                dto_rust_type: "String",
                ts_type: None,
                form: FormKind::Text,
            },
            Case {
                name: "title",
                spec: "string",
                col_type: "StringNull",
                dto_rust_type: "Option<String>",
                ts_type: None,
                form: FormKind::Text,
            },
            Case {
                name: "title",
                spec: "string^",
                col_type: "StringUniq",
                dto_rust_type: "String",
                ts_type: None,
                form: FormKind::Text,
            },
            // text
            Case {
                name: "body",
                spec: "text!",
                col_type: "Text",
                dto_rust_type: "String",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "body",
                spec: "text",
                col_type: "TextNull",
                dto_rust_type: "Option<String>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            // uuid
            Case {
                name: "pid",
                spec: "uuid!",
                col_type: "Uuid",
                dto_rust_type: "Uuid",
                ts_type: Some("string"),
                form: FormKind::Text,
            },
            Case {
                name: "pid",
                spec: "uuid",
                col_type: "UuidNull",
                dto_rust_type: "Option<Uuid>",
                ts_type: Some("string | null"),
                form: FormKind::Text,
            },
            Case {
                name: "pid",
                spec: "uuid^",
                col_type: "UuidUniq",
                dto_rust_type: "Uuid",
                ts_type: Some("string"),
                form: FormKind::Text,
            },
            // small_int
            Case {
                name: "rank",
                spec: "small_int!",
                col_type: "SmallInteger",
                dto_rust_type: "i16",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "rank",
                spec: "small_int",
                col_type: "SmallIntegerNull",
                dto_rust_type: "Option<i16>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
            // int -- the deliberate 1.0 fix: 32-bit, not BigInteger
            Case {
                name: "hits",
                spec: "int!",
                col_type: "Integer",
                dto_rust_type: "i32",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "hits",
                spec: "int",
                col_type: "IntegerNull",
                dto_rust_type: "Option<i32>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
            // big_int -- 64-bit
            Case {
                name: "views",
                spec: "big_int!",
                col_type: "BigInteger",
                dto_rust_type: "i64",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "views",
                spec: "big_int",
                col_type: "BigIntegerNull",
                dto_rust_type: "Option<i64>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
            // small_unsigned
            Case {
                name: "small_qty",
                spec: "small_unsigned!",
                col_type: "SmallUnsigned",
                dto_rust_type: "i16",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "small_qty",
                spec: "small_unsigned",
                col_type: "SmallUnsignedNull",
                dto_rust_type: "Option<i16>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
            // big_unsigned
            Case {
                name: "big_qty",
                spec: "big_unsigned!",
                col_type: "BigUnsigned",
                dto_rust_type: "i64",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "big_qty",
                spec: "big_unsigned",
                col_type: "BigUnsignedNull",
                dto_rust_type: "Option<i64>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
            // float
            Case {
                name: "weight",
                spec: "float!",
                col_type: "Float",
                dto_rust_type: "f32",
                ts_type: None,
                form: FormKind::Number,
            },
            Case {
                name: "weight",
                spec: "float",
                col_type: "FloatNull",
                dto_rust_type: "Option<f32>",
                ts_type: None,
                form: FormKind::Number,
            },
            // double
            Case {
                name: "precise_weight",
                spec: "double!",
                col_type: "Double",
                dto_rust_type: "f64",
                ts_type: None,
                form: FormKind::Number,
            },
            Case {
                name: "precise_weight",
                spec: "double",
                col_type: "DoubleNull",
                dto_rust_type: "Option<f64>",
                ts_type: None,
                form: FormKind::Number,
            },
            // decimal
            Case {
                name: "price",
                spec: "decimal!",
                col_type: "Decimal",
                dto_rust_type: "Decimal",
                ts_type: Some("string"),
                form: FormKind::Number,
            },
            Case {
                name: "price",
                spec: "decimal",
                col_type: "DecimalNull",
                dto_rust_type: "Option<Decimal>",
                ts_type: Some("string | null"),
                form: FormKind::Number,
            },
            // decimal_len
            Case {
                name: "price",
                spec: "decimal_len:10:2!",
                col_type: "DecimalLen(10, 2)",
                dto_rust_type: "Decimal",
                ts_type: Some("string"),
                form: FormKind::Number,
            },
            Case {
                name: "price",
                spec: "decimal_len:10:2",
                col_type: "DecimalLenNull(10, 2)",
                dto_rust_type: "Option<Decimal>",
                ts_type: Some("string | null"),
                form: FormKind::Number,
            },
            Case {
                name: "price",
                spec: "decimal_len:10:2^",
                col_type: "DecimalLenUniq(10, 2)",
                dto_rust_type: "Decimal",
                ts_type: Some("string"),
                form: FormKind::Number,
            },
            // money
            Case {
                name: "amount",
                spec: "money!",
                col_type: "Money",
                dto_rust_type: "Decimal",
                ts_type: Some("string"),
                form: FormKind::Number,
            },
            Case {
                name: "amount",
                spec: "money",
                col_type: "MoneyNull",
                dto_rust_type: "Option<Decimal>",
                ts_type: Some("string | null"),
                form: FormKind::Number,
            },
            // bool (no unique variant exists)
            Case {
                name: "active",
                spec: "bool!",
                col_type: "Boolean",
                dto_rust_type: "bool",
                ts_type: None,
                form: FormKind::Checkbox,
            },
            Case {
                name: "active",
                spec: "bool",
                col_type: "BooleanNull",
                dto_rust_type: "Option<bool>",
                ts_type: None,
                form: FormKind::Checkbox,
            },
            // date
            Case {
                name: "born_on",
                spec: "date!",
                col_type: "Date",
                dto_rust_type: "Date",
                ts_type: Some("string"),
                form: FormKind::DateTime,
            },
            Case {
                name: "born_on",
                spec: "date",
                col_type: "DateNull",
                dto_rust_type: "Option<Date>",
                ts_type: Some("string | null"),
                form: FormKind::DateTime,
            },
            // time
            Case {
                name: "starts_at",
                spec: "time!",
                col_type: "Time",
                dto_rust_type: "Time",
                ts_type: Some("string"),
                form: FormKind::DateTime,
            },
            Case {
                name: "starts_at",
                spec: "time",
                col_type: "TimeNull",
                dto_rust_type: "Option<Time>",
                ts_type: Some("string | null"),
                form: FormKind::DateTime,
            },
            // date_time
            Case {
                name: "posted_at",
                spec: "date_time!",
                col_type: "DateTime",
                dto_rust_type: "DateTime",
                ts_type: Some("string"),
                form: FormKind::DateTime,
            },
            Case {
                name: "posted_at",
                spec: "date_time",
                col_type: "DateTimeNull",
                dto_rust_type: "Option<DateTime>",
                ts_type: Some("string | null"),
                form: FormKind::DateTime,
            },
            // tstz (no unique variant exists)
            Case {
                name: "happened_at",
                spec: "tstz!",
                col_type: "TimestampWithTimeZone",
                dto_rust_type: "DateTimeWithTimeZone",
                ts_type: Some("string"),
                form: FormKind::DateTime,
            },
            Case {
                name: "happened_at",
                spec: "tstz",
                col_type: "TimestampWithTimeZoneNull",
                dto_rust_type: "Option<DateTimeWithTimeZone>",
                ts_type: Some("string | null"),
                form: FormKind::DateTime,
            },
            // json
            Case {
                name: "payload",
                spec: "json!",
                col_type: "Json",
                dto_rust_type: "serde_json::Value",
                ts_type: Some("unknown"),
                form: FormKind::Textarea,
            },
            Case {
                name: "payload",
                spec: "json",
                col_type: "JsonNull",
                dto_rust_type: "Option<serde_json::Value>",
                ts_type: Some("unknown | null"),
                form: FormKind::Textarea,
            },
            // jsonb
            Case {
                name: "payload",
                spec: "jsonb!",
                col_type: "JsonBinary",
                dto_rust_type: "serde_json::Value",
                ts_type: Some("unknown"),
                form: FormKind::Textarea,
            },
            Case {
                name: "payload",
                spec: "jsonb",
                col_type: "JsonBinaryNull",
                dto_rust_type: "Option<serde_json::Value>",
                ts_type: Some("unknown | null"),
                form: FormKind::Textarea,
            },
            // blob
            Case {
                name: "data",
                spec: "blob!",
                col_type: "Blob",
                dto_rust_type: "Vec<u8>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "data",
                spec: "blob",
                col_type: "BlobNull",
                dto_rust_type: "Option<Vec<u8>>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            // var_binary
            Case {
                name: "data",
                spec: "var_binary:16!",
                col_type: "VarBinary(16)",
                dto_rust_type: "Vec<u8>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "data",
                spec: "var_binary:16",
                col_type: "VarBinaryNull(16)",
                dto_rust_type: "Option<Vec<u8>>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            // binary_len
            Case {
                name: "data",
                spec: "binary_len:16!",
                col_type: "BinaryLen(16)",
                dto_rust_type: "Vec<u8>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "data",
                spec: "binary_len:16",
                col_type: "BinaryLenNull(16)",
                dto_rust_type: "Option<Vec<u8>>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            // enum -- dto type derives from the column name
            Case {
                name: "status",
                spec: "enum:draft,published!",
                col_type: "String",
                dto_rust_type: "Status",
                ts_type: None,
                form: FormKind::Select(vec!["draft".to_string(), "published".to_string()]),
            },
            Case {
                name: "status",
                spec: "enum:draft,published",
                col_type: "StringNull",
                dto_rust_type: "Option<Status>",
                ts_type: None,
                form: FormKind::Select(vec!["draft".to_string(), "published".to_string()]),
            },
            // array
            Case {
                name: "tags",
                spec: "array:string!",
                col_type: "array(ArrayColType::String)",
                dto_rust_type: "Vec<String>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "tags",
                spec: "array:string",
                col_type: "array_null(ArrayColType::String)",
                dto_rust_type: "Option<Vec<String>>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "tags",
                spec: "array:string^",
                col_type: "array_uniq(ArrayColType::String)",
                dto_rust_type: "Vec<String>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            Case {
                name: "scores",
                spec: "array:big_int!",
                col_type: "array(ArrayColType::BigInt)",
                dto_rust_type: "Vec<i64>",
                ts_type: None,
                form: FormKind::Textarea,
            },
            // references
            Case {
                name: "user",
                spec: "references",
                col_type: "BigInteger",
                dto_rust_type: "i64",
                ts_type: Some("number"),
                form: FormKind::Number,
            },
            Case {
                name: "user",
                spec: "references?",
                col_type: "BigInteger",
                dto_rust_type: "Option<i64>",
                ts_type: Some("number | null"),
                form: FormKind::Number,
            },
        ];

        for case in cases {
            let c = col(case.name, case.spec);
            assert_eq!(
                c.col_type(),
                case.col_type,
                "col_type mismatch for `{}:{}`",
                case.name,
                case.spec
            );
            assert_eq!(
                c.dto_rust_type(),
                case.dto_rust_type,
                "dto_rust_type mismatch for `{}:{}`",
                case.name,
                case.spec
            );
            assert_eq!(
                c.ts_type(),
                case.ts_type.map(std::string::ToString::to_string),
                "ts_type mismatch for `{}:{}`",
                case.name,
                case.spec
            );
            assert_eq!(
                c.form_input(),
                case.form,
                "form_input mismatch for `{}:{}`",
                case.name,
                case.spec
            );
        }
    }

    #[test]
    fn unsigned_scalar_type_derivations_are_reachable_even_though_dsl_does_not_expose_it() {
        // `ScalarType::Unsigned` is part of the type model (mirroring
        // `schema.rs`'s `ColType::Unsigned`) but the `unsigned` DSL keyword
        // is kept as an alias of `big_unsigned` to preserve the retired
        // field-type-mapping JSON's exact semantics (see
        // `scalar_from_base_name`). Directly construct a `Column` to verify
        // the variant's own derivations still hold, so the type stays
        // exercised even though nothing in the parser can reach it today.
        let required = Column {
            name: "small_qty".to_string(),
            kind: ColumnKind::Scalar(ScalarType::Unsigned),
            nullable: false,
            unique: false,
        };
        assert_eq!(required.col_type(), "Unsigned");
        assert_eq!(required.dto_rust_type(), "u32");
        assert_eq!(required.ts_type(), Some("number".to_string()));
        assert_eq!(required.form_input(), FormKind::Number);

        let nullable = Column {
            nullable: true,
            ..required
        };
        assert_eq!(nullable.col_type(), "UnsignedNull");
        assert_eq!(nullable.dto_rust_type(), "Option<u32>");
        assert_eq!(nullable.ts_type(), Some("number | null".to_string()));

        let unique = Column {
            name: "small_qty".to_string(),
            kind: ColumnKind::Scalar(ScalarType::Unsigned),
            nullable: false,
            unique: true,
        };
        assert_eq!(unique.col_type(), "UnsignedUniq");
    }

    // ---- columns_from_fields ------------------------------------------------

    fn field(name: &str, spec: &str) -> (String, String) {
        (name.to_string(), spec.to_string())
    }

    #[test]
    fn columns_from_fields_parses_each_field() {
        let fields = vec![field("title", "string!"), field("user", "references")];
        let cols = columns_from_fields(&fields).expect("failed to parse fields");
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].name, "title");
        assert_eq!(cols[0].kind, ColumnKind::Scalar(ScalarType::String));
        assert_eq!(
            cols[1].kind,
            ColumnKind::Reference {
                target: "user".to_string(),
                fk_field: None
            }
        );
    }

    #[test]
    fn columns_from_fields_skips_ignore_fields() {
        let fields = vec![
            field("title", "string!"),
            field("created_at", "date_time"),
            field("updated_at", "date_time"),
            field("create_at", "date_time"),
            field("update_at", "date_time"),
        ];
        let cols = columns_from_fields(&fields).expect("failed to parse fields");
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].name, "title");
    }

    #[test]
    fn columns_from_fields_propagates_parse_errors() {
        assert!(columns_from_fields(&[field("thing", "not_a_real_type")]).is_err());
    }

    #[test]
    fn columns_from_fields_empty_input_is_empty_output() {
        assert_eq!(columns_from_fields(&[]).expect("failed to parse fields"), vec![]);
    }
}
