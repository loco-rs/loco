use heck::ToSnakeCase;
use sea_orm::{
    sea_query::{
        Alias, ColumnDef, Expr, Index, IntoIden, PgInterval, Table, TableAlterStatement,
        TableCreateStatement, TableForeignKey,
    },
    ColumnType, ConnectionTrait, DbErr, ForeignKeyAction,
};
pub use sea_orm_migration::schema::*;
use sea_orm_migration::{prelude::Iden, sea_query, SchemaManager};

#[derive(Iden)]
enum GeneralIds {
    CreatedAt,
    UpdatedAt,
}

/// Alter table
pub fn alter<T: IntoIden + 'static>(name: T) -> TableAlterStatement {
    Table::alter().table(name).take()
}

/// Wrapping table schema creation.
pub fn table_auto_tz<T>(name: T) -> TableCreateStatement
where
    T: IntoIden + 'static,
{
    timestamps_tz(Table::create().table(name).if_not_exists().take())
}

// these two are just aliases, original types exist in seaorm already.

#[must_use]
pub fn timestamps_tz(t: TableCreateStatement) -> TableCreateStatement {
    let mut t = t;
    t.col(timestamp_with_time_zone(GeneralIds::CreatedAt).default(Expr::current_timestamp()))
        .col(timestamp_with_time_zone(GeneralIds::UpdatedAt).default(Expr::current_timestamp()));
    t.take()
}

/// Create a nullable timestamptz column definition.
pub fn timestamptz_null<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .timestamp_with_time_zone()
        .null()
        .take()
}

/// Create a non-nullable timestamptz column definition.
pub fn timestamptz<T>(name: T) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .timestamp_with_time_zone()
        .not_null()
        .take()
}

/// Create a non-nullable enum column definition.
pub fn enum_type<T>(name: T, enum_name: &str) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .enumeration::<Alias, Alias, Vec<Alias>>(Alias::new(enum_name), vec![])
        .not_null()
        .take()
}

/// Create a nullable enum column definition.
pub fn enum_type_null<T>(name: T, enum_name: &str) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .enumeration::<Alias, Alias, Vec<Alias>>(Alias::new(enum_name), vec![])
        .null()
        .take()
}

/// Create a non-nullable enum column definition with default value.
///
/// # Example
/// ```ignore
/// create_table(m, "users", vec![
///     ("status", ColType::EnumWithDefault("status_enum".to_string(), vec!["pending".to_string(), "active".to_string()], "pending".to_string()))
/// ], vec![]).await;
/// ```
pub fn enum_type_with_default<T>(name: T, enum_name: &str, default_value: &str) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .enumeration::<Alias, Alias, Vec<Alias>>(Alias::new(enum_name), vec![])
        .not_null()
        .default(Expr::val(default_value))
        .take()
}

/// Create a nullable enum column definition with default value.
///
/// # Example
/// ```ignore
/// create_table(m, "users", vec![
///     ("status", ColType::EnumNullWithDefault("status_enum".to_string(), vec!["pending".to_string(), "active".to_string()], "pending".to_string()))
/// ], vec![]).await;
/// ```
pub fn enum_type_null_with_default<T>(name: T, enum_name: &str, default_value: &str) -> ColumnDef
where
    T: IntoIden,
{
    ColumnDef::new(name)
        .enumeration::<Alias, Alias, Vec<Alias>>(Alias::new(enum_name), vec![])
        .null()
        .default(Expr::val(default_value))
        .take()
}

/// Check if an enum type already exists in the database
async fn check_enum_exists(m: &SchemaManager<'_>, enum_name: &str) -> Result<bool, DbErr> {
    match m.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => {
            let query = format!(
                "SELECT EXISTS (
                    SELECT 1 FROM pg_type 
                    WHERE typname = '{enum_name}' 
                    AND typtype = 'e'
                )"
            );

            let result = m
                .get_connection()
                .query_one(sea_orm::Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    query,
                ))
                .await?;

            Ok(result.is_some_and(|row| row.try_get::<bool>("", "exists").unwrap_or(false)))
        }
        sea_orm::DatabaseBackend::Sqlite => {
            // SQLite doesn't have native enum types, so we'll always return false
            // to allow creation of enum-like behavior through CHECK constraints
            Ok(false)
        }
        sea_orm::DatabaseBackend::MySql => {
            // MySQL doesn't support enums in the same way, so we'll always return false
            Ok(false)
        }
    }
}

#[derive(Debug)]
pub enum ColType {
    PkAuto,
    PkUuid,
    CharLen(u32),
    CharLenWithDefault(u32, char),
    CharLenNull(u32),
    CharLenUniq(u32),
    Char,
    CharWithDefault(char),
    CharNull,
    CharUniq,
    StringLen(u32),
    StringLenWithDefault(u32, String),
    StringLenNull(u32),
    StringLenUniq(u32),
    String,
    StringWithDefault(String),
    StringNull,
    StringUniq,
    Text,
    TextWithDefault(String),
    TextNull,
    TextUniq,
    Integer,
    IntegerWithDefault(i32),
    IntegerNull,
    IntegerUniq,
    Unsigned,
    UnsignedWithDefault(u32),
    UnsignedNull,
    UnsignedUniq,
    SmallUnsigned,
    SmallUnsignedWithDefault(u16),
    SmallUnsignedNull,
    SmallUnsignedUniq,
    BigUnsigned,
    BigUnsignedWithDefault(u64),
    BigUnsignedNull,
    BigUnsignedUniq,
    SmallInteger,
    SmallIntegerWithDefault(i16),
    SmallIntegerNull,
    SmallIntegerUniq,
    BigInteger,
    BigIntegerWithDefault(i64),
    BigIntegerNull,
    BigIntegerUniq,
    Decimal,
    DecimalWithDefault(f64),
    DecimalNull,
    DecimalUniq,
    DecimalLen(u32, u32),
    DecimalLenWithDefault(u32, u32, f64),
    DecimalLenNull(u32, u32),
    DecimalLenUniq(u32, u32),
    Float,
    FloatWithDefault(f32),
    FloatNull,
    FloatUniq,
    Double,
    DoubleWithDefault(f64),
    DoubleNull,
    DoubleUniq,
    Boolean,
    BooleanWithDefault(bool),
    BooleanNull,
    Date,
    DateWithDefault(String),
    DateNull,
    DateUniq,
    DateTime,
    DateTimeWithDefault(String),
    DateTimeNull,
    DateTimeUniq,
    Time,
    TimeWithDefault(String),
    TimeNull,
    TimeUniq,
    Interval(Option<PgInterval>, Option<u32>),
    IntervalNull(Option<PgInterval>, Option<u32>),
    IntervalUniq(Option<PgInterval>, Option<u32>),
    Binary,
    BinaryNull,
    BinaryUniq,
    BinaryLen(u32),
    BinaryLenNull(u32),
    BinaryLenUniq(u32),
    VarBinary(u32),
    VarBinaryNull(u32),
    VarBinaryUniq(u32),
    TimestampWithTimeZone,
    TimestampWithTimeZoneWithDefault(String),
    TimestampWithTimeZoneNull,
    Json,
    JsonNull,
    JsonUniq,
    JsonBinary,
    JsonBinaryNull,
    JsonBinaryUniq,
    Blob,
    BlobNull,
    BlobUniq,
    Money,
    MoneyWithDefault(f64),
    MoneyNull,
    MoneyUniq,
    Uuid,
    UuidNull,
    UuidUniq,
    VarBitLen(u32),
    VarBitLenNull(u32),
    VarBitLenUniq(u32),
    Array(ColumnType),
    ArrayNull(ColumnType),
    ArrayUniq(ColumnType),
    // Enum types
    Enum(String, Vec<String>),
    EnumNull(String, Vec<String>),
    EnumWithDefault(String, Vec<String>, String),
    EnumNullWithDefault(String, Vec<String>, String),
}

pub enum ArrayColType {
    String,
    Int,
    BigInt,
    Float,
    Double,
    Bool,
}

impl ColType {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn array(kind: ArrayColType) -> Self {
        Self::Array(Self::array_col_type(&kind))
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn array_uniq(kind: ArrayColType) -> Self {
        Self::ArrayUniq(Self::array_col_type(&kind))
    }

    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn array_null(kind: ArrayColType) -> Self {
        Self::ArrayNull(Self::array_col_type(&kind))
    }

    fn array_col_type(kind: &ArrayColType) -> ColumnType {
        match kind {
            ArrayColType::String => ColumnType::string(None),
            ArrayColType::Int => ColumnType::Integer,
            ArrayColType::BigInt => ColumnType::BigInteger,
            ArrayColType::Float => ColumnType::Float,
            ArrayColType::Double => ColumnType::Double,
            ArrayColType::Bool => ColumnType::Boolean,
        }
    }
}

impl ColType {
    #[allow(clippy::too_many_lines)]
    fn to_def(&self, name: impl IntoIden) -> ColumnDef {
        match self {
            Self::PkAuto => pk_auto(name),
            Self::PkUuid => pk_uuid(name),
            Self::CharLen(len) => char_len(name, *len),
            Self::CharLenNull(len) => char_len_null(name, *len),
            Self::CharLenUniq(len) => char_len_uniq(name, *len),
            Self::Char => char(name),
            Self::CharNull => char_null(name),
            Self::CharUniq => char_uniq(name),
            Self::StringLen(len) => string_len(name, *len),
            Self::StringLenNull(len) => string_len_null(name, *len),
            Self::StringLenUniq(len) => string_len_uniq(name, *len),
            Self::String => string(name),
            Self::StringNull => string_null(name),
            Self::StringUniq => string_uniq(name),
            Self::Text => text(name),
            Self::TextNull => text_null(name),
            Self::TextUniq => text_uniq(name),
            Self::Integer => integer(name),
            Self::IntegerNull => integer_null(name),
            Self::IntegerUniq => integer_uniq(name),
            // Self::TinyInteger => tiny_integer(name),
            // Self::TinyIntegerNull => tiny_integer_null(name),
            // Self::TinyIntegerUniq => tiny_integer_uniq(name),
            Self::Unsigned => unsigned(name),
            Self::UnsignedNull => unsigned_null(name),
            Self::UnsignedUniq => unsigned_uniq(name),
            // Self::TinyUnsigned => tiny_unsigned(name),
            // Self::TinyUnsignedNull => tiny_unsigned_null(name),
            // Self::TinyUnsignedUniq => tiny_unsigned_uniq(name),
            Self::SmallUnsigned => small_unsigned(name),
            Self::SmallUnsignedNull => small_unsigned_null(name),
            Self::SmallUnsignedUniq => small_unsigned_uniq(name),
            Self::BigUnsigned => big_unsigned(name),
            Self::BigUnsignedNull => big_unsigned_null(name),
            Self::BigUnsignedUniq => big_unsigned_uniq(name),
            Self::SmallInteger => small_integer(name),
            Self::SmallIntegerNull => small_integer_null(name),
            Self::SmallIntegerUniq => small_integer_uniq(name),
            Self::BigInteger => big_integer(name),
            Self::BigIntegerNull => big_integer_null(name),
            Self::BigIntegerUniq => big_integer_uniq(name),
            Self::Decimal => decimal(name),
            Self::DecimalNull => decimal_null(name),
            Self::DecimalUniq => decimal_uniq(name),
            Self::DecimalLen(precision, scale) => decimal_len(name, *precision, *scale),
            Self::DecimalLenNull(precision, scale) => decimal_len_null(name, *precision, *scale),
            Self::DecimalLenUniq(precision, scale) => decimal_len_uniq(name, *precision, *scale),
            Self::Float => float(name),
            Self::FloatNull => float_null(name),
            Self::FloatUniq => float_uniq(name),
            Self::Double => double(name),
            Self::DoubleNull => double_null(name),
            Self::DoubleUniq => double_uniq(name),
            Self::Boolean => boolean(name),
            Self::BooleanNull => boolean_null(name),
            // Self::Timestamp => timestamp(name),
            // Self::TimestampNull => timestamp_null(name),
            // Self::TimestampUniq => timestamp_uniq(name),
            Self::Date => date(name),
            Self::DateNull => date_null(name),
            Self::DateUniq => date_uniq(name),
            Self::DateTime => date_time(name),
            Self::DateTimeNull => date_time_null(name),
            Self::DateTimeUniq => date_time_uniq(name),
            Self::Time => time(name),
            Self::TimeNull => time_null(name),
            Self::TimeUniq => time_uniq(name),
            Self::Interval(ival, prec) => interval(name, ival.clone(), *prec),
            Self::IntervalNull(ival, prec) => interval_null(name, ival.clone(), *prec),
            Self::IntervalUniq(ival, prec) => interval_uniq(name, ival.clone(), *prec),
            Self::Binary => binary(name),
            Self::BinaryNull => binary_null(name),
            Self::BinaryUniq => binary_uniq(name),
            Self::BinaryLen(len) => binary_len(name, *len),
            Self::BinaryLenNull(len) => binary_len_null(name, *len),
            Self::BinaryLenUniq(len) => binary_len_uniq(name, *len),
            Self::VarBinary(len) => var_binary(name, *len),
            Self::VarBinaryNull(len) => var_binary_null(name, *len),
            Self::VarBinaryUniq(len) => var_binary_uniq(name, *len),
            Self::TimestampWithTimeZone => timestamptz(name),
            Self::TimestampWithTimeZoneNull => timestamptz_null(name),
            Self::Json => json(name),
            Self::JsonNull => json_null(name),
            Self::JsonUniq => json_uniq(name),
            Self::JsonBinary => json_binary(name),
            Self::JsonBinaryNull => json_binary_null(name),
            Self::JsonBinaryUniq => json_binary_uniq(name),
            Self::Blob => blob(name),
            Self::BlobNull => blob_null(name),
            Self::BlobUniq => blob_uniq(name),
            Self::Money => money(name),
            Self::MoneyNull => money_null(name),
            Self::MoneyUniq => money_uniq(name),
            Self::Uuid => uuid(name),
            Self::UuidNull => uuid_null(name),
            Self::UuidUniq => uuid_uniq(name),
            Self::VarBitLen(len) => varbit(name, *len),
            Self::VarBitLenNull(len) => varbit_null(name, *len),
            Self::VarBitLenUniq(len) => varbit_uniq(name, *len),
            Self::Array(kind) => array(name, kind.clone()),
            Self::ArrayNull(kind) => array_null(name, kind.clone()),
            Self::ArrayUniq(kind) => array_uniq(name, kind.clone()),
            // Enum types
            Self::Enum(enum_name, _) => enum_type(name, enum_name),
            Self::EnumNull(enum_name, _) => enum_type_null(name, enum_name),
            Self::EnumWithDefault(enum_name, _, default_value) => {
                enum_type_with_default(name, enum_name, default_value)
            }
            Self::EnumNullWithDefault(enum_name, _, default_value) => {
                enum_type_null_with_default(name, enum_name, default_value)
            }
            // defaults
            Self::MoneyWithDefault(v) => money(name).default(*v).take(),
            Self::IntegerWithDefault(v) => integer(name).default(*v).take(),
            Self::UnsignedWithDefault(v) => unsigned(name).default(*v).take(),
            Self::SmallUnsignedWithDefault(v) => small_unsigned(name).default(*v).take(),
            Self::BigUnsignedWithDefault(v) => big_unsigned(name).default(*v).take(),
            Self::SmallIntegerWithDefault(v) => small_integer(name).default(*v).take(),
            Self::BigIntegerWithDefault(v) => big_integer(name).default(*v).take(),
            Self::DecimalWithDefault(v) => decimal(name).default(*v).take(),
            Self::DecimalLenWithDefault(p, s, v) => decimal_len(name, *p, *s).default(*v).take(),
            Self::FloatWithDefault(v) => float(name).default(*v).take(),
            Self::DoubleWithDefault(v) => double(name).default(*v).take(),
            Self::BooleanWithDefault(v) => boolean(name).default(*v).take(),
            Self::DateWithDefault(v) => date(name).default(v.clone()).take(),
            Self::DateTimeWithDefault(v) => date_time(name).default(v.clone()).take(),
            Self::TimeWithDefault(v) => time(name).default(v.clone()).take(),
            Self::TimestampWithTimeZoneWithDefault(v) => {
                timestamptz(name).default(v.clone()).take()
            }
            Self::CharWithDefault(v) => char(name).default(*v).take(),
            Self::CharLenWithDefault(len, v) => char_len(name, *len).default(*v).take(),
            Self::StringWithDefault(v) => string(name).default(v.clone()).take(),
            Self::StringLenWithDefault(len, v) => string_len(name, *len).default(v.clone()).take(),
            Self::TextWithDefault(v) => text(name).default(v.clone()).take(),
        }
    }
}

///
/// Create a table.
/// ```ignore
/// create_table(m, "movies", vec![
///     ("title", ColType::String)
/// ],
/// vec![]
/// )
/// .await;
/// ```
///
/// ```shell
/// loco g migration CreateMovies title:string user:references
/// loco g migration CreateMovies title:string user:references:admin_id
/// ```
/// # Errors
/// fails when it fails
pub async fn create_table(
    m: &SchemaManager<'_>,
    table: &str,
    cols: &[(&str, ColType)],
    refs: &[(&str, &str)], // [(from_tbl, to_tbl), ...]
) -> Result<(), DbErr> {
    create_table_impl(m, table, cols, refs, false, true).await
}

///
/// Create a join table. A join table has a composite primary key.
/// ```ignore
/// create_join_table(m, "movies", vec![
///     ("title", ColType::String)
/// ],
/// vec![]
/// )
/// .await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn create_join_table(
    m: &SchemaManager<'_>,
    table: &str,
    cols: &[(&str, ColType)],
    refs: &[(&str, &str)], // [(from_tbl, to_tbl), ...]
) -> Result<(), DbErr> {
    create_table_impl(m, table, cols, refs, true, true).await
}

/// Create a table without automatic timestamps.
/// This gives users full control over their table schema.
/// ```ignore
/// create_table_without_timestamps(m, "movies", vec![
///     ("title", ColType::String)
/// ],
/// vec![]
/// )
/// .await;
/// ```
///
/// ```shell
/// loco g migration CreateMovies title:string user:references --without-timestamps
/// ```
/// # Errors
/// fails when it fails
pub async fn create_table_without_timestamps(
    m: &SchemaManager<'_>,
    table: &str,
    cols: &[(&str, ColType)],
    refs: &[(&str, &str)], // [(from_tbl, to_tbl), ...]
) -> Result<(), DbErr> {
    create_table_impl(m, table, cols, refs, false, false).await
}

/// Create a join table without automatic timestamps.
/// A join table has a composite primary key.
/// ```ignore
/// create_join_table_without_timestamps(m, "movies", vec![
///     ("title", ColType::String)
/// ],
/// vec![]
/// )
/// .await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn create_join_table_without_timestamps(
    m: &SchemaManager<'_>,
    table: &str,
    cols: &[(&str, ColType)],
    refs: &[(&str, &str)], // [(from_tbl, to_tbl), ...]
) -> Result<(), DbErr> {
    create_table_impl(m, table, cols, refs, true, false).await
}

async fn create_table_impl(
    m: &SchemaManager<'_>,
    table: &str,
    cols: &[(&str, ColType)],
    refs: &[(&str, &str)], // [(from_tbl, to_tbl), ...]
    is_join: bool,
    add_timestamps: bool, // New parameter to control timestamp addition
) -> Result<(), DbErr> {
    let nz_table = normalize_table(table);

    // Create enum types automatically if they don't exist
    let mut enum_types = std::collections::HashSet::new();
    for (_, col_type) in cols {
        match col_type {
            ColType::Enum(enum_name, variants)
            | ColType::EnumNull(enum_name, variants)
            | ColType::EnumWithDefault(enum_name, variants, _)
            | ColType::EnumNullWithDefault(enum_name, variants, _) => {
                if !enum_types.contains(enum_name) {
                    enum_types.insert(enum_name.clone());

                    // Check if enum type already exists
                    let enum_exists = check_enum_exists(m, enum_name).await?;

                    if !enum_exists {
                        // Create enum type with provided variants
                        match m.get_database_backend() {
                            sea_orm::DatabaseBackend::Postgres => {
                                let variant_aliases: Vec<Alias> =
                                    variants.iter().map(Alias::new).collect();
                                m.create_type(
                                    sea_query::extension::postgres::Type::create()
                                        .as_enum(Alias::new(enum_name))
                                        .values(variant_aliases)
                                        .to_owned(),
                                )
                                .await?;
                            }
                            #[allow(clippy::match_same_arms)]
                            sea_orm::DatabaseBackend::Sqlite => {
                                // SQLite doesn't support native enum types
                                // The enum behavior will be handled by the column definition
                                // which will create a TEXT column with CHECK constraints
                            }
                            sea_orm::DatabaseBackend::MySql => {
                                // MySql not supporting
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Conditionally create table with or without timestamps
    let mut stmt = if add_timestamps {
        table_auto_tz(Alias::new(&nz_table))
    } else {
        Table::create()
            .table(Alias::new(&nz_table))
            .if_not_exists()
            .take()
    };

    if is_join {
        let mut idx = Index::create();
        idx.name(format!("idx-{nz_table}-refs-pk"))
            .table(Alias::new(&nz_table));

        for (from_tbl, ref_name) in refs {
            let nz_from_table = normalize_table(from_tbl);
            // in movies, user:references, creates a `user_id` field or what ever in
            // `ref_name` if given
            let nz_ref_name = if ref_name.is_empty() {
                reference_id(&nz_from_table)
            } else {
                (*ref_name).to_string()
            };
            idx.col(Alias::new(nz_ref_name));
        }
        stmt.primary_key(&mut idx);
    }

    for (name, atype) in cols {
        stmt.col(atype.to_def(Alias::new(*name)));
    }

    // user, None
    // users, None
    // user, admin_id
    for (from_tbl, ref_name) in refs {
        // Check for nullable reference
        let (nz_from_table, is_nullable) = from_tbl.strip_suffix('?').map_or_else(
            || (normalize_table(from_tbl), false),
            |stripped| (normalize_table(stripped), true),
        );
        let nz_ref_name = if ref_name.is_empty() {
            reference_id(&nz_from_table)
        } else {
            (*ref_name).to_string()
        };
        // Only add the column if it doesn't already exist in cols
        if !cols.iter().any(|(col_name, _)| *col_name == nz_ref_name) {
            let col_type = if is_nullable {
                ColType::IntegerNull
            } else {
                ColType::Integer
            };
            stmt.col(col_type.to_def(Alias::new(&nz_ref_name)));
        }
        // Set FK actions based on nullability
        let mut fk = sea_query::ForeignKey::create();
        fk.name(format!("fk-{nz_from_table}-{nz_ref_name}-to-{nz_table}"));
        fk.from(Alias::new(&nz_table), Alias::new(&nz_ref_name));
        fk.to(Alias::new(nz_from_table), Alias::new("id"));
        if is_nullable {
            fk.on_delete(ForeignKeyAction::SetNull);
            fk.on_update(ForeignKeyAction::NoAction);
        } else {
            fk.on_delete(ForeignKeyAction::Cascade);
            fk.on_update(ForeignKeyAction::Cascade);
        }
        stmt.foreign_key(&mut fk);
    }
    m.create_table(stmt).await?;
    Ok(())
}

/// person -> people, movies -> movie
fn normalize_table(table: &str) -> String {
    cruet::to_plural(table).to_snake_case()
}

/// users -> `user_id`
fn reference_id(totbl: &str) -> String {
    format!("{}_id", cruet::to_singular(totbl).to_snake_case())
}

///
/// Add a column to a table with a column type.
///
/// ```ignore
/// add_column(m, "movies", "title", ColType::String).await;
/// ```
/// # Errors
/// fails when it fails
pub async fn add_column(
    m: &SchemaManager<'_>,
    table: &str,
    name: &str,
    atype: ColType,
) -> Result<(), DbErr> {
    let nz_table = normalize_table(table);
    m.alter_table(
        alter(Alias::new(nz_table))
            .add_column(atype.to_def(Alias::new(name)))
            .to_owned(),
    )
    .await?;
    Ok(())
}

///
/// Drop a column from a table.
///
/// ```ignore
/// drop_column(m, "movies", "title").await;
/// ```
/// # Errors
/// fails when it fails
pub async fn remove_column(m: &SchemaManager<'_>, table: &str, name: &str) -> Result<(), DbErr> {
    let nz_table = normalize_table(table);
    m.alter_table(
        alter(Alias::new(nz_table))
            .drop_column(Alias::new(name))
            .to_owned(),
    )
    .await?;
    Ok(())
}

///
/// Adds a reference. Reads "movies belongs-to users":
/// ```ignore
/// add_reference(m, "movies", "users").await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn add_reference(
    m: &SchemaManager<'_>,
    fromtbl: &str,
    totbl: &str,
    refname: &str,
) -> Result<(), DbErr> {
    // movies
    let nz_fromtbl = normalize_table(fromtbl);
    // users
    let nz_totbl = normalize_table(totbl);
    // user_id
    let nz_ref_name = if refname.is_empty() {
        reference_id(totbl)
    } else {
        refname.to_string()
    };
    let bk = m.get_database_backend();
    let col = ColType::Integer.to_def(Alias::new(&nz_ref_name));
    let fk = TableForeignKey::new()
        // fk-movies-user_id-to-users
        .name(format!("fk-{nz_fromtbl}-{nz_ref_name}-to-{nz_totbl}"))
        // from movies#user_id
        .from_tbl(Alias::new(&nz_fromtbl))
        .from_col(Alias::new(&nz_ref_name)) // xxx fix
        // to users#id
        .to_tbl(Alias::new(nz_totbl))
        .to_col(Alias::new("id"))
        .on_delete(ForeignKeyAction::Cascade)
        .on_update(ForeignKeyAction::Cascade)
        .to_owned();
    match bk {
        sea_orm::DatabaseBackend::MySql | sea_orm::DatabaseBackend::Postgres => {
            // from movies to users -> movies#user_id to users#id
            m.alter_table(
                alter(Alias::new(&nz_fromtbl))
                    // add movies#user_id (the user_id column is new)
                    .add_column(col.clone()) // XXX fix, totbl_id
                    // add fk on movies#user_id
                    .add_foreign_key(&fk)
                    .to_owned(),
            )
            .await?;
        }
        sea_orm::DatabaseBackend::Sqlite => {
            // from movies to users -> movies#user_id to users#id
            m.alter_table(
                alter(Alias::new(&nz_fromtbl))
                    // add movies#user_id (the user_id column is new)
                    .add_column(col.clone()) // XXX fix, totbl_id
                    .to_owned(),
            )
            .await?;
            // Per Rails 5.2, adding FK to existing table does nothing because
            // sqlite will not allow it. FK in sqlite are applied only on table
            // creation. more: https://www.bigbinary.com/blog/rails-6-adds-add_foreign_key-and-remove_foreign_key-for-sqlite3
            // we comment it below leaving it for academic purposes.
            /*
                m.alter_table(
                    alter(Alias::new(&nz_fromtbl))
                        // add fk on movies#user_id
                        .add_foreign_key(&fk)
                        .to_owned(),
                )
                .await?;
            */
        }
    }
    Ok(())
}

///
/// Removes a reference by constructing its name from the table names.
/// ```ignore
/// remove_reference(m, "movies", "users").await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn remove_reference(
    m: &SchemaManager<'_>,
    fromtbl: &str,
    totbl: &str,
    refname: &str,
) -> Result<(), DbErr> {
    // movies
    let nz_fromtbl = normalize_table(fromtbl);
    // users
    let nz_totbl = normalize_table(totbl);
    // user_id
    let nz_ref_name = if refname.is_empty() {
        reference_id(totbl)
    } else {
        refname.to_string()
    };
    let bk = m.get_database_backend();
    match bk {
        sea_orm::DatabaseBackend::MySql | sea_orm::DatabaseBackend::Postgres => {
            // from movies to users -> movies#user_id to users#id
            m.alter_table(
                alter(Alias::new(&nz_fromtbl))
                    .drop_foreign_key(
                        // fk-movies-user_id-to-users
                        Alias::new(format!("fk-{nz_fromtbl}-{nz_ref_name}-to-{nz_totbl}")),
                    )
                    .to_owned(),
            )
            .await?;
        }
        sea_orm::DatabaseBackend::Sqlite => {
            // Per Rails 5.2, removing FK on existing table does nothing because
            // sqlite will not allow it.
            // more: https://www.bigbinary.com/blog/rails-6-adds-add_foreign_key-and-remove_foreign_key-for-sqlite3
        }
    }
    Ok(())
}

///
/// Drop a table
/// ```ignore
/// drop_table(m, "movies").await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn drop_table(m: &SchemaManager<'_>, table: &str) -> Result<(), DbErr> {
    let nz_table = normalize_table(table);
    m.drop_table(Table::drop().table(Alias::new(nz_table)).to_owned())
        .await
}

///
/// Add enum values to an existing enum type
/// ```ignore
/// add_enum_values(m, "status_enum", vec!["suspended", "cancelled"]).await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn add_enum_values(
    m: &SchemaManager<'_>,
    enum_name: &str,
    new_values: Vec<String>,
) -> Result<(), DbErr> {
    match m.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => {
            for value in new_values {
                m.get_connection()
                    .execute(sea_orm::Statement::from_string(
                        sea_orm::DatabaseBackend::Postgres,
                        format!("ALTER TYPE {enum_name} ADD VALUE '{value}'"),
                    ))
                    .await?;
            }
        }
        sea_orm::DatabaseBackend::Sqlite => {
            // SQLite doesn't support native enums, values are handled by CHECK constraints
            tracing::info!(
                "SQLite: Enum values are handled by CHECK constraints. No action needed."
            );
        }
        sea_orm::DatabaseBackend::MySql => {
            // MySQL handles enums differently
            tracing::info!(
                "MySQL: Enum values are handled by column definition. No action needed."
            );
        }
    }
    Ok(())
}

///
/// Drop an enum type completely
/// ```ignore
/// drop_enum_type(m, "status_enum").await;
/// ```
///
/// # Errors
/// fails when it fails
pub async fn drop_enum_type(m: &SchemaManager<'_>, enum_name: &str) -> Result<(), DbErr> {
    match m.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => {
            // First check if the enum type exists
            let enum_exists = check_enum_exists(m, enum_name).await?;
            if !enum_exists {
                tracing::info!("Enum type '{}' does not exist, skipping drop", enum_name);
                return Ok(());
            }

            // Try to drop the enum type with CASCADE to handle any remaining references
            let query = format!("DROP TYPE IF EXISTS {enum_name} CASCADE");
            m.get_connection()
                .execute(sea_orm::Statement::from_string(
                    sea_orm::DatabaseBackend::Postgres,
                    query,
                ))
                .await?;
        }
        _ => {
            // SQLite/MySQL don't have native enum types
            tracing::info!("No native enum type to drop for this database backend.");
        }
    }
    Ok(())
}
