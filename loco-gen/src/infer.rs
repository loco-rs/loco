use cruet::{case::snake::to_snake_case, Inflector};

use crate::{Error, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum MigrationType {
    CreateTable { table: String },
    AddColumns { table: String },
    RemoveColumns { table: String },
    AddReference { table: String },
    CreateJoinTable { table_a: String, table_b: String },
    Empty,
}

pub enum FieldType {
    Reference,
    ReferenceWithCustomField(String),
    Type(String),
    TypeWithParameters(String, Vec<String>),
}

pub fn parse_field_type(ftype: &str) -> Result<FieldType> {
    let parts: Vec<&str> = ftype.split(':').collect();

    match parts.as_slice() {
        ["references"] => Ok(FieldType::Reference),
        ["references", f] => Ok(FieldType::ReferenceWithCustomField((*f).to_string())),
        [t] => Ok(FieldType::Type((*t).to_string())),
        [t, params @ ..] => Ok(FieldType::TypeWithParameters(
            (*t).to_string(),
            params.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )),
        [] => Err(Error::Message(format!("cannot parse type: `{ftype}`"))),
    }
}
pub fn guess_migration_type(migration_name: &str) -> MigrationType {
    let normalized_name = to_snake_case(migration_name);
    let parts: Vec<&str> = normalized_name.split('_').collect();

    match parts.as_slice() {
        ["create", table_name] => MigrationType::CreateTable {
            table: table_name.to_plural(),
        },
        ["add", _reference_name, "ref", "to", table_name] => MigrationType::AddReference {
            table: table_name.to_plural(),
        },
        ["add", _column_names @ .., "to", table_name] => MigrationType::AddColumns {
            table: table_name.to_plural(),
        },
        ["remove", _column_names @ .., "from", table_name] => MigrationType::RemoveColumns {
            table: table_name.to_plural(),
        },
        ["create", "join", "table", table_a, "and", table_b] => {
            let table_a = table_a.to_singular();
            let table_b = table_b.to_singular();
            MigrationType::CreateJoinTable { table_a, table_b }
        }
        _ => MigrationType::Empty,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_create_table() {
        assert_eq!(
            guess_migration_type("CreateUsers"),
            MigrationType::CreateTable {
                table: "users".to_string(),
            }
        );
    }

    #[test]
    fn test_infer_add_columns() {
        assert_eq!(
            guess_migration_type("AddNameAndAgeToUsers"),
            MigrationType::AddColumns {
                table: "users".to_string(),
            }
        );
    }

    #[test]
    fn test_infer_remove_columns() {
        assert_eq!(
            guess_migration_type("RemoveNameAndAgeFromUsers"),
            MigrationType::RemoveColumns {
                table: "users".to_string(),
            }
        );
    }

    #[test]
    fn test_infer_add_reference() {
        assert_eq!(
            guess_migration_type("AddUserRefToPosts"),
            MigrationType::AddReference {
                table: "posts".to_string(),
            }
        );
    }

    #[test]
    fn test_infer_create_join_table() {
        assert_eq!(
            guess_migration_type("CreateJoinTableUsersAndGroups"),
            MigrationType::CreateJoinTable {
                table_a: "user".to_string(),
                table_b: "group".to_string()
            }
        );
    }

    #[test]
    fn test_empty_migration() {
        assert_eq!(
            guess_migration_type("UnknownMigrationType"),
            MigrationType::Empty
        );
    }
}
