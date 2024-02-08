+++
title = "Migration"
description = ""
date = 2021-05-01T18:10:00+00:00
updated = 2021-05-01T18:10:00+00:00
draft = false
weight = 12
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

Loco is a migration-first framework, similar to Rails. Which means that when you want to add models, data fields, or model oriented changes - you start with a migration that describes it, and then you apply the migration to get back generated entities in `model/_entities`.

This enforces _everything-as-code_, _reproducibility_ and _atomicity_, where no knowledge of the schema goes missing. 


## Verbs, singular and plural

* **references**: use **singular** for the table name, and a `:references` type. `user:references` (references `Users`), `vote:references` (references `Votes`)
* **column names**: anything you like. Prefer `snake_case`
* **table names**: **plural, snake case**. `users`, `draft_posts`.
* **migration names**: anything that can be a file name, prefer snake case. `create_table_users`, `add_vote_id_to_movies`
* **model names**: generated automatically for you. Usually the generated name is pascal case, plural. `Users`, `UsersVotes`
 
Here are some examples showcasing the naming conventions:

```sh
$ cargo loco generate model movies long_title:string user:references
```

* model name in plural: `movies`
* reference user is in singular: `user:references`
* column name in snake case: `long_title:string`

### Naming migrations

There are no rules for how to name migrations, but here's a few guidelines to keep your migration stack readable as a list of files:

* `<table>` - create a table, plural, `movies`
* `add_<table>_<field>` - add a column, `add_users_email`
* `index_<table>_<field>` - add an index, `index_users_email`
* `alter_` - change a schema, `alter_users`
* `delete_<table>_<field>` - remove a column, `delete_users_email`
* `data_fix_` - fix some data, using entity queries or raw SQL, `data_fix_users_timezone_issue_315`

Example:

```sh
$ cargo loco generate migration add_users_email
```

## Creating a table

Prefer going through the new model generator:

```
$ cargo loco generate model notes title:string
```

See more in [Models](@/docs/the-app/models.md)

## Creating a migration

For changing tables, adding columns, altering tables or applying data fixes, we can generate a migration.


```
$ cargo loco generate migration <name of migration>
```


### Add or remove a column

Adding a column:

```rust
  manager
    .alter_table(
        Table::alter()
            .table(Movies::Table)
            .add_column_if_not_exists(integer(Movies::Rating))
            .to_owned(),
    )
    .await
```

Dropping a column:

```rust
  manager
    .alter_table(
        Table::alter()
            .table(Movies::Table)
            .drop_column(Movies::Rating)
            .to_owned(),
    )
    .await
```

### Add index


You can copy some of this code for adding an index

```rust
  manager
    .create_index(
        Index::create()
            .name("idx-movies-rating")
            .table(Movies::Table)
            .col(Movies::Rating)
            .to_owned(),
    )
    .await;
```

### Create a data fix


Creating a data fix in a migration is easy - just `use` your models as you would otherwise:

```rust
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

    let db = manager.get_connection();

    cake::ActiveModel {
        name: Set("Cheesecake".to_owned()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
  }
```

Having said that, it's up to you to code your data fixes in a `task` or `migration` or an ad-hoc `playground`.


## Relationships

### One to many

Here is how to associate a `Company` with an existing `User` model.

```
$ cargo loco generate model company name:string user:references
```

This will create a migration with a `user_id` field in `Company` which will reference a `User`.


### Many to many

Here is how to create a typical "votes" table, which links a `User` and a `Movie` with a many-to-many link table. Note that it uses the special `--link` flag in the model generator.

Let's create a new `Movie` entity:

```
$ cargo loco generate model movies title:string
```

And now the link table between `User` (which we already have) and `Movie` (which we just generated) to record votes:

```
$ cargo loco generate model --link users_votes user:references movie:references vote:int
    ..
    ..
Writing src/models/_entities/movies.rs
Writing src/models/_entities/notes.rs
Writing src/models/_entities/users.rs
Writing src/models/_entities/mod.rs
Writing src/models/_entities/prelude.rs
... Done.
```

This will create a many-to-many link table named `UsersVotes` with a composite primary key containing both `user_id` and `movie_id`. Because it has precisely 2 IDs, SeaORM will identify it as a many-to-many link table, and generate entities with the appropriate `via()` relationship:


```rust
// User, newly generated entity with a `via` relation at _entities/users.rs

// ..
impl Related<super::movies::Entity> for Entity {
    fn to() -> RelationDef {
        super::users_votes::Relation::Movies.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::users_votes::Relation::Users.def().rev())
    }
}
```

Using `via()` will cause `find_related` to walk through the link table without you needing to know the details of the link table.

