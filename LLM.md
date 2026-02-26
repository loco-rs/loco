# Loco Framework - LLM Context Document

## Overview
Loco is a Rust web framework inspired by Rails, designed for rapid development with built-in generators and CLI tools. The framework follows a structured MVC approach where CLI tools scaffold the initial project structure, then local CLI commands generate specific components.

**Key Principles:**
- **Fat models, slim controllers**: Models contain business logic, controllers handle HTTP routing
- **Convention over configuration**: Standardized folder structure and naming conventions
- **CLI-driven development**: Generate components over manual coding
- **Migration-first**: Database schema changes start with migrations

## LLM Instruction

* In all cases you MUST start a new loco app using the loco CLI (loco new).
* if you need to add code, you MUST first explore a way to do it with the local project CLI (cargo loco)
* when you're writing code yourself, make sure you put the code in the right app layer and follow the conventions in a strict way
* before reinventing the wheel you MUST explore the Loco framework itself and see what building blocks it offers for you to use instead of write yourself
* beware of CLI commands that will require user interaction - always prefer non-interactive sessions and explore the CLI flags that will let you do so
* when faced with user authentication requirements - always try to user the included User auth abilities Loco provides
* when generating models using the CLI (via model or scaffold) you do not need to manually specify the `created_at` or `updated_at` fields as they are built in

## CLI Command Structure
Loco uses a hierarchical CLI structure:
- **`loco new`**: Creates new projects (external tool)
- **`cargo loco`**: Local project CLI with subcommands:
  - **`cargo loco g`** or **`cargo loco generate`**: Generate components (models, controllers, etc.)
  - **`cargo loco db`**: Database operations (migrate, entities, reset)
  - **`cargo loco start`**: Start the application
  - **`cargo loco watch`**: Watch and restart the application
  - **`cargo loco routes`**: Display all registered routes
  - **`cargo loco doctor`**: Validate app configuration and connections

## Core Architecture

### Project Structure
```
src/
├── app.rs              # Main application entry point
├── controllers/        # HTTP request handlers
├── models/            # Data models and database entities
├── views/             # Response templates and views
├── initializers/      # App initialization logic
├── mailers/           # Email functionality
├── workers/           # Background job processors
├── tasks/             # CLI tasks
└── lib.rs             # Library exports
```

### Key Components
- **Controllers**: Handle HTTP requests using Axum, manage routing and request/response flow
- **Models**: SeaORM entities with automatic migrations, contain business logic and database operations
- **Views**: JSON response structs and Tera templates for HTML responses
- **Workers**: Background job processing with Redis/PostgreSQL/SQLite support
- **Mailers**: Email templating and sending functionality
- **Tasks**: CLI commands for app management and ad-hoc operations
- **Data**: Static data loaders for configuration and read-only data

## Installation and Dependencies

### Required Dependencies

#### 1. Loco CLI (Project Generator)
```bash
# Install the Loco CLI for creating new projects
cargo install loco
```

#### 2. SeaORM CLI (Database Operations)
```bash
# Install SeaORM CLI for database migrations and schema management
# Only needed when working with databases
cargo install sea-orm-cli
```

**Note**: The SeaORM CLI is only required if you're using database features. If you're building a lightweight service without a database, you can skip this installation.

### Optional Dependencies

#### Development Tools
```bash
# Auto-reload during development (optional)
cargo install watchexec

# Alternative development runner (optional)
cargo install bacon
```

### Verification
```bash
# Check if Loco CLI is installed
loco --version

# Check if SeaORM CLI is installed (if using databases)
sea-orm-cli --version
```

## CLI Tools

### 1. Project Generation (`loco new`)
```bash
# Create new project (interactive)
loco new

# Create new project with specific name
loco new my-app

# Available starters:
# - SaaS App with client side rendering
# - SaaS App with server side rendering  
# - Rest API
# - Lightweight Service
```

**Available Options:**
- **Database**: SQLite (default), PostgreSQL
- **Background Workers**: Async (in-process), Redis, PostgreSQL, SQLite
- **Asset Serving**: Client-side (React/Vite), Server-side (Tera templates)
- **Authentication**: Built-in JWT authentication with SaaS starter

### 2. Local Project CLI (`cargo loco`)
```bash
# Generate controller
cargo loco generate controller users index show create update destroy

# Generate model
cargo loco g model article title:string content:text author:references

# Generate scaffold (model + controller + views)
cargo loco g scaffold post title:string body:text published:bool
```

**Note**: The CLI supports both `cargo loco generate` and `cargo loco g` (shorthand) for most commands.

## Generation Commands

### Controller Generation
```bash
cargo run --bin my-app-cli generate controller <name> <actions...>
```
**Actions**: index, show, create, update, destroy, custom_action_name

**Types:**
- `api`: JSON API endpoints
- `html`: Traditional web pages
- `htmx`: HTMX-enabled pages

### Model Generation
```bash
cargo loco g model <name> <field:type...>
```

**Field Types:**
- `string`: String field (nullable by default)
- `string!`: Required string field
- `string^`: Unique string field

**Note**: Field types follow the pattern `type[modifier]` where:
- No modifier = nullable field (e.g., `string` = nullable, `int` = nullable)
- `!` = required field (e.g., `string!` = required, `int!` = required)
- `^` = unique field (e.g., `string^` = unique, `int^` = unique)
- `?` = nullable (used only with references, e.g., `user:references?`)
- `text`: Long text field (nullable by default)
- `text!`: Required text field
- `text^`: Unique text field
- `int`: Integer field (nullable by default)
- `int!`: Required integer field
- `int^`: Unique integer field
- `big_int`: 64-bit integer (nullable by default)
- `big_int!`: Required 64-bit integer
- `big_int^`: Unique 64-bit integer
- `small_int`: 16-bit integer (nullable by default)
- `small_int!`: Required 16-bit integer
- `small_int^`: Unique 16-bit integer
- `unsigned`: Unsigned integer (nullable by default)
- `unsigned!`: Required unsigned integer
- `unsigned^`: Unique unsigned integer
- `small_unsigned`: 16-bit unsigned integer (nullable by default)
- `small_unsigned!`: Required 16-bit unsigned integer
- `small_unsigned^`: Unique 16-bit unsigned integer
- `big_unsigned`: 64-bit unsigned integer (nullable by default)
- `big_unsigned!`: Required 64-bit unsigned integer
- `big_unsigned^`: Unique 64-bit unsigned integer
- `float`: 32-bit float (nullable by default)
- `float!`: Required 32-bit float
- `float^`: Unique 32-bit float
- `double`: 64-bit float (nullable by default)
- `double!`: Required 64-bit float
- `double^`: Unique 64-bit float
- `bool`: Boolean field (nullable by default)
- `bool!`: Required boolean field
- `decimal`: Decimal field (nullable by default)
- `decimal!`: Required decimal field
- `decimal_len`: Decimal with precision/scale (nullable by default, requires 2 parameters)
- `decimal_len!`: Required decimal with precision/scale (requires 2 parameters)
- `ts`: Timestamp (nullable by default)
- `ts!`: Required timestamp
- `tstz`: Timestamp with timezone (nullable by default)
- `tstz!`: Required timestamp with timezone
- `date`: Date field (nullable by default)
- `date!`: Required date field
- `date^`: Unique date field
- `date_time`: DateTime field (nullable by default)
- `date_time!`: Required DateTime field
- `date_time^`: Unique DateTime field
- `uuid`: UUID field (nullable by default)
- `uuid!`: Required UUID field
- `uuid^`: Unique UUID field
- `json`: JSON field (nullable by default)
- `json!`: Required JSON field
- `json^`: Unique JSON field
- `jsonb`: JSONB field (nullable by default)
- `jsonb!`: Required JSONB field
- `jsonb^`: Unique JSONB field
- `blob`: Binary data (nullable by default)
- `blob!`: Required binary data
- `blob^`: Unique binary data
- `binary_len`: Binary data with length (nullable by default, requires 1 parameter)
- `binary_len!`: Required binary data with length (requires 1 parameter)
- `binary_len^`: Unique binary data with length (requires 1 parameter)
- `var_binary`: Variable-length binary data (nullable by default, requires 1 parameter)
- `var_binary!`: Required variable-length binary data (requires 1 parameter)
- `var_binary^`: Unique variable-length binary data (requires 1 parameter)
- `money`: Money field (nullable by default)
- `money!`: Required money field
- `money^`: Unique money field
- `array`: Array of strings (nullable by default)
- `array!`: Array of strings (required)
- `array^`: Array of strings (unique)

**Examples:**
```bash
# Basic model
cargo loco g model user name:string! email:string^ age:int

# With references
cargo loco g model post title:string! content:text! author:references

# With nullable references
cargo loco g model comment body:text! post:references? user:references?

# Complex model with various types
cargo loco g model product name:string! description:text price:decimal! category:references tags:array! published:bool! created_at:ts!
```

**Default Fields:**
When generating a model, Loco automatically adds these fields:
- `id`: Auto-incrementing primary key
- `created_at`: Timestamp when record was created (required, `ts!` type)
- `updated_at`: Timestamp when record was updated (required, `ts!` type)

**Migration Process:**
1. **Generate model**: Creates migration file
2. **Apply migration**: `cargo loco db migrate`
3. **Generate entities**: `cargo loco db entities` (creates SeaORM entities)
4. **Model file**: Generated in `src/models/` for your business logic

## Database Entities and Relations

### Reference Types
```bash
# Basic foreign key reference
cargo loco g model post title:string! author:references

# Nullable foreign key reference
cargo loco g model comment body:text! post:references? user:references?

# Custom foreign key field name
cargo loco g model order total:decimal! customer:references:customer_id
```

**How References Work:**
- **Field name determines the referenced table**: `author:references` → references the `authors` table
- **Foreign key column naming**: Automatically creates `{singular_field_name}_id` (e.g., `author_id`)
- **Table name inference**: Uses the field name as the table name (singularized)
- **Custom column names**: `user:references:author_id` → creates `author_id` column instead of `user_id`

**Examples:**
- `author:references` → `author_id` column → references `authors.id`
- `post:references` → `post_id` column → references `posts.id`
- `user:references` → `user_id` column → references `users.id`
- `category:references` → `category_id` column → references `categories.id`
- `user:references:author_id` → `author_id` column → references `users.id`
- `user:references:editor_id` → `editor_id` column → references `users.id`
- `user:references:admin_id` → `admin_id` column → references `users.id`

**Table Naming Rules:**
- **Model names**: Use singular nouns (`user`, `post`, `category`)
- **Table names**: Automatically pluralized (`users`, `posts`, `categories`)
- **Foreign key columns**: `{singular_model_name}_id` (`user_id`, `post_id`, `category_id`)
- **Reference inference**: Field name becomes the table name (singularized)

**Complete Example:**
```bash
# Generate user model
cargo loco g model user name:string! email:string^

# Generate post model with author reference
cargo loco g model post title:string! content:text! author:references

# Generate post model with custom column name
cargo loco g model post title:string! content:text! user:references:author_id
```

**What Gets Created:**
1. **Users table**: `users` with columns `id`, `name`, `email`
2. **Posts table**: `posts` with columns `id`, `title`, `content`, `author_id`
3. **Foreign key**: `posts.author_id` → `users.id`
4. **Migration**: Automatically creates the foreign key constraint

**With Custom Column Name (`user:references:author_id`):**
1. **Users table**: `users` with columns `id`, `name`, `email`
2. **Posts table**: `posts` with columns `id`, `title`, `content`, `author_id`
3. **Foreign key**: `posts.author_id` → `users.id`
4. **Migration**: Automatically creates the foreign key constraint

**Reference Syntax:**
- **Basic**: `field:references` → references `{field}s` table, creates `{field}_id` column
- **Custom column name**: `field:references:custom_name` → references `{field}s` table, creates `custom_name` column

**Nullable References:**
- `field:references?` → creates nullable foreign key with `ON DELETE SET NULL`
- `field:references?:custom_name` → nullable reference with custom column name

### Many-to-Many Relationships
```bash
# Create join table for many-to-many
cargo loco g migration create_join_table_users_and_groups

# Generate models for the relationship
cargo loco g model user_group user:references group:references role:string! joined_at:tstz!
```

### Array Fields
```bash
# Array of strings
cargo loco g model article title:string! tags:array categories:array!

# Array of strings
cargo loco g model product name:string! sizes:array prices:array!

# Array of strings
cargo loco g model user name:string! permissions:array
```

### Advanced Field Types
```bash
# Decimal with precision/scale
cargo loco g model order total:decimal_len!:10:2 currency:string!

# Binary data
cargo loco g model file name:string! content:blob! mime_type:string!

# JSON fields
cargo loco g model config key:string! value:json! metadata:jsonb
```

### Model Structure Patterns

#### Basic Entity Model
```rust
use async_trait::async_trait;
use loco_rs::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateParams {
    pub title: String,
    pub content: String,
    pub author_id: i32,
}

impl Model {
    pub async fn create_post(db: &DatabaseConnection, params: CreateParams) -> ModelResult<Self> {
        let post = posts::ActiveModel {
            title: ActiveValue::set(params.title),
            content: ActiveValue::set(params.content),
            author_id: ActiveValue::set(params.author_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
        
        Ok(post)
    }
    
    pub async fn find_by_author(db: &DatabaseConnection, author_id: i32) -> ModelResult<Vec<Self>> {
        let posts = posts::Entity::find()
            .filter(posts::Column::AuthorId.eq(author_id))
            .all(db)
            .await?;
            
        Ok(posts)
    }
}
```

#### Model with Relations
```rust
impl Model {
    pub async fn find_with_author(db: &DatabaseConnection, id: i32) -> ModelResult<Option<Self>> {
        let post = posts::Entity::find_by_id(id)
            .find_also_related(users::Entity)
            .one(db)
            .await?;
            
        Ok(post.map(|(post, user)| (post, user)))
    }
    
    pub async fn find_with_comments(db: &DatabaseConnection, id: i32) -> ModelResult<Option<Self>> {
        let post = posts::Entity::find_by_id(id)
            .find_with(comments::Relation::Comments.def())
            .one(db)
            .await?;
            
        Ok(post)
    }
}
```

#### Validation and Business Logic
```rust
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateParams {
    #[validate(length(min = 3, max = 100))]
    pub title: String,
    
    #[validate(length(min = 10))]
    pub content: String,
    
    #[validate(range(min = 1))]
    pub author_id: i32,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(CreateParams {
            title: self.title.as_ref().unwrap_or_default().to_string(),
            content: self.content.as_ref().unwrap_or_default().to_string(),
            author_id: self.author_id.as_ref().unwrap_or_default(),
        })
    }
}
```

### Migration Patterns

#### Basic Table Creation
```rust
use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "posts",
            &[
                ("id", ColType::PkAuto),
                ("title", ColType::String),
                ("content", ColType::Text),
                ("author_id", ColType::Integer),
                ("published", ColType::Boolean),
                ("created_at", ColType::TimestampWithTimeZone),
                ("updated_at", ColType::TimestampWithTimeZone),
            ],
            &[],
        )
        .await?;
        
        // Add foreign key constraint
        create_foreign_key(
            m,
            ForeignKey::create()
                .name("fk_posts_author_id")
                .from(posts::Table, posts::Column::AuthorId)
                .to(users::Table, users::Column::Id)
                .on_delete(ForeignKeyAction::Cascade)
                .on_update(ForeignKeyAction::Cascade),
        )
        .await?;
        
        Ok(())
    }
}
```

#### Join Table Migration
```rust
async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
    create_table(
        m,
        "user_groups",
        &[
            ("id", ColType::PkAuto),
            ("user_id", ColType::Integer),
            ("group_id", ColType::Integer),
            ("role", ColType::String),
            ("joined_at", ColType::TimestampWithTimeZone),
        ],
        &[],
    )
    .await?;
    
    // Add foreign keys
    create_foreign_key(
        m,
        ForeignKey::create()
            .name("fk_user_groups_user_id")
            .from(user_groups::Table, user_groups::Column::UserId)
            .to(users::Table, users::Column::Id),
    )
    .await?;
    
    create_foreign_key(
        m,
        ForeignKey::create()
            .name("fk_user_groups_group_id")
            .from(user_groups::Table, user_groups::Column::GroupId)
            .to(groups::Table, groups::Column::Id),
    )
    .await?;
    
    Ok(())
}
```

## Common Database Patterns

### One-to-Many Relationships
```bash
# User has many Posts
cargo loco g model user name:string! email:string^
cargo loco g model post title:string! content:text! author:references

# Category has many Products
cargo loco g model category name:string! description:text
cargo loco g model product name:string! price:decimal! category:references
```

### Many-to-Many Relationships
```bash
# Users can belong to multiple Groups
cargo loco g model group name:string! description:text
cargo loco g model user_group user:references group:references role:string! joined_at:tstz!

# Products can have multiple Tags
cargo loco g model tag name:string! color:string
cargo loco g model product_tag product:references tag:references
```

### Self-Referential Relationships
```bash
# Comments can have parent comments (threading)
cargo loco g model comment body:text! author:references post:references parent:references?

# Categories can have parent categories (hierarchy)
cargo loco g model category name:string! parent:references? level:int!
```

### Audit and Tracking Fields
```bash
# Models with audit fields (automatically handled by Loco)
cargo loco g model document title:string! content:text! author:references
# Automatically adds: created_at, updated_at

# Custom audit fields
cargo loco g model article title:string! content:text! author:references published_at:tstz? archived_at:tstz?
```

### Soft Deletes
```bash
# Models with soft delete capability
cargo loco g model post title:string! content:text! author:references deleted_at:tstz?
```

### Polymorphic Associations
```bash
# Generic commentable content
cargo loco g model comment body:text! author:references commentable_type:string! commentable_id:int!

# Generic attachments
cargo loco g model attachment filename:string! file_path:string! attachable_type:string! attachable_id:int!
```

## Field Type Modifiers

### Nullability
- **Default**: Fields are nullable (`string` → `Option<String>`)
- **Required**: Add `!` (`string!` → `String`)
- **Unique**: Add `^` (`string^` → `String` with unique constraint)

### Array Types
- **Nullable array**: `array` → `Option<Vec<String>>`
- **Required array**: `array!` → `Vec<String>`
- **Unique array**: `array^` → `Vec<String>` with unique constraint

### Decimal Precision
- **Basic decimal**: `decimal` → `Option<Decimal>`
- **With precision/scale**: `decimal_len:10:2` → `Option<Decimal>` with 10 digits, 2 decimal places

### Timestamps
- **Basic timestamp**: `tstz` → `Option<DateTimeWithTimeZone>`
- **Required timestamp**: `tstz!` → `DateTimeWithTimeZone`
- **Date only**: `date` → `Option<Date>`
- **DateTime**: `date_time` → `Option<DateTime>`

## Best Practices

### 1. Naming Conventions
- **Models**: Singular nouns (`user`, `post`, `category`)
- **Tables**: Plural nouns (automatically generated: `users`, `posts`, `categories`)
- **Foreign keys**: `{model_name}_id` (automatically generated)

### 2. Field Design
- Always use `string!` for required text fields
- Use `text` for long content (blog posts, descriptions)
- Use `int` for small numbers, `big_int` for large numbers
- Use `decimal` for monetary values
- Use `uuid` for public identifiers, `int` for internal IDs

### 3. Relationships
- Use `references` for required foreign keys
- Use `references?` for optional foreign keys
- Create join tables for many-to-many relationships
- Use descriptive names for join tables

### 4. Validation
- Implement `Validatable` trait for all models
- Use validator crate attributes for field validation
- Add business logic validation in model methods

### Scaffold Generation
```bash
cargo loco g scaffold <name> <field:type...>
```
Generates complete CRUD: model, controller, views, and routes.

### Migration Generation
```bash
cargo loco g migration <name>
```
Creates new database migration files.

**Migration Naming Conventions:**
- **Create table**: `CreatePosts` → creates `posts` table
- **Add columns**: `AddNameAndAgeToUsers` → adds columns to `users` table
- **Remove columns**: `RemoveNameAndAgeFromUsers` → removes columns from `users` table
- **Add reference**: `AddUserRefToPosts` → adds foreign key to `posts` table
- **Create join table**: `CreateJoinTableUsersAndGroups` → creates join table `users_groups`
- **Empty migration**: `FixUsersTable` → creates blank migration for custom edits

## Migration Authoring

Loco provides powerful helper functions for writing migrations manually. These functions handle database-specific differences and provide a consistent API across PostgreSQL, SQLite, and MySQL.

### Core Migration Functions

#### Table Operations
```rust
use loco_rs::schema::*;

// Create a new table with automatic timestamps
create_table(m, "posts", &[
    ("title", ColType::String),
    ("content", ColType::Text),
    ("published", ColType::Boolean),
], &[
    ("user", ""),  // Creates user_id column
    ("user", "category_id"),  // Custom column name
]).await?;

// Create a join table with composite primary key
create_join_table(m, "posts_tags", &[
    ("created_at", ColType::TimestampWithTimeZone),
], &[
    ("post", ""),  // Creates post_id column
    ("tag", ""),   // Creates tag_id column
]).await?;

// Drop a table
drop_table(m, "old_posts").await?;
```

#### Column Operations
```rust
// Add a column
add_column(m, "posts", "excerpt", ColType::Text).await?;

// Remove a column
remove_column(m, "posts", "excerpt").await?;

// Add a reference (foreign key)
add_reference(m, "posts", "users", "author_id").await?;

// Remove a reference
remove_reference(m, "posts", "users", "author_id").await?;
```

### Column Type Helpers

#### Basic Types
```rust
// String types
ColType::String           // VARCHAR (not null)
ColType::StringNull       // VARCHAR (nullable)
ColType::StringUniq       // VARCHAR UNIQUE
ColType::StringLen(100)   // VARCHAR(100)
ColType::Text             // TEXT (not null)
ColType::TextNull         // TEXT (nullable)
ColType::TextUniq         // TEXT UNIQUE

// Numeric types
ColType::Integer          // INTEGER (not null)
ColType::IntegerNull      // INTEGER (nullable)
ColType::IntegerUniq      // INTEGER UNIQUE
ColType::SmallInteger     // SMALLINT (not null)
ColType::SmallIntegerNull // SMALLINT (nullable)
ColType::SmallIntegerUniq // SMALLINT UNIQUE
ColType::BigInteger       // BIGINT (not null)
ColType::BigIntegerNull   // BIGINT (nullable)
ColType::BigIntegerUniq   // BIGINT UNIQUE
ColType::Unsigned         // UNSIGNED INT (not null)
ColType::UnsignedNull     // UNSIGNED INT (nullable)
ColType::UnsignedUniq     // UNSIGNED INT UNIQUE
ColType::SmallUnsigned    // SMALLINT UNSIGNED (not null)
ColType::SmallUnsignedNull // SMALLINT UNSIGNED (nullable)
ColType::SmallUnsignedUniq // SMALLINT UNSIGNED UNIQUE
ColType::BigUnsigned      // BIGINT UNSIGNED (not null)
ColType::BigUnsignedNull  // BIGINT UNSIGNED (nullable)
ColType::BigUnsignedUniq  // BIGINT UNSIGNED UNIQUE

// Float types
ColType::Float            // FLOAT (not null)
ColType::FloatNull        // FLOAT (nullable)
ColType::FloatUniq        // FLOAT UNIQUE
ColType::Double           // DOUBLE (not null)
ColType::DoubleNull       // DOUBLE (nullable)
ColType::DoubleUniq       // DOUBLE UNIQUE

// Decimal types
ColType::Decimal          // DECIMAL (not null)
ColType::DecimalNull      // DECIMAL (nullable)
ColType::DecimalUniq      // DECIMAL UNIQUE
ColType::DecimalLen(10, 2) // DECIMAL(10,2) (not null)
ColType::DecimalLenNull(10, 2) // DECIMAL(10,2) (nullable)
ColType::DecimalLenUniq(10, 2) // DECIMAL(10,2) UNIQUE
ColType::DecimalLenWithDefault(10, 2, 99.99) // DECIMAL(10,2) DEFAULT 99.99

// Boolean types
ColType::Boolean          // BOOLEAN (not null)
ColType::BooleanNull      // BOOLEAN (nullable)
ColType::BooleanUniq      // BOOLEAN UNIQUE
ColType::BooleanWithDefault(true) // BOOLEAN DEFAULT true

// Date/Time types
ColType::Date             // DATE (not null)
ColType::DateNull         // DATE (nullable)
ColType::DateUniq         // DATE UNIQUE
ColType::DateTime         // DATETIME (not null)
ColType::DateTimeNull     // DATETIME (nullable)
ColType::DateTimeUniq     // DATETIME UNIQUE
ColType::TimestampWithTimeZone // TIMESTAMPTZ (not null)
ColType::TimestampWithTimeZoneNull // TIMESTAMPTZ (nullable)
ColType::Time             // TIME (not null)
ColType::TimeNull         // TIME (nullable)
ColType::TimeUniq         // TIME UNIQUE

// Binary types
ColType::Blob             // BLOB (not null)
ColType::BlobNull         // BLOB (nullable)
ColType::BlobUniq         // BLOB UNIQUE
ColType::BinaryLen(1000)  // BINARY(1000) (not null)
ColType::BinaryLenNull(1000) // BINARY(1000) (nullable)
ColType::BinaryLenUniq(1000) // BINARY(1000) UNIQUE
ColType::VarBinary(1000)  // VARBINARY(1000) (not null)
ColType::VarBinaryNull(1000) // VARBINARY(1000) (nullable)
ColType::VarBinaryUniq(1000) // VARBINARY(1000) UNIQUE

// JSON types
ColType::Json             // JSON (not null)
ColType::JsonNull         // JSON (nullable)
ColType::JsonUniq         // JSON UNIQUE
ColType::JsonBinary       // JSONB (PostgreSQL, not null)
ColType::JsonBinaryNull   // JSONB (PostgreSQL, nullable)
ColType::JsonBinaryUniq   // JSONB (PostgreSQL, UNIQUE)

// UUID types
ColType::Uuid             // UUID (not null)
ColType::UuidNull         // UUID (nullable)
ColType::UuidUniq         // UUID UNIQUE

// Money types
ColType::Money            // MONEY (not null)
ColType::MoneyNull        // MONEY (nullable)
ColType::MoneyUniq        // MONEY UNIQUE
ColType::MoneyWithDefault(0.0) // MONEY DEFAULT 0.0

// Array types
ColType::Array(ArrayColType::String)     // ARRAY[TEXT] (not null)
ColType::ArrayNull(ArrayColType::String) // ARRAY[TEXT] (nullable)
ColType::ArrayUniq(ArrayColType::String) // ARRAY[TEXT] UNIQUE
ColType::Array(ArrayColType::Int)        // ARRAY[INTEGER] (not null)
ColType::Array(ArrayColType::BigInt)     // ARRAY[BIGINT] (not null)
ColType::Array(ArrayColType::Float)      // ARRAY[FLOAT] (not null)
ColType::Array(ArrayColType::Double)     // ARRAY[DOUBLE] (not null)
ColType::Array(ArrayColType::Bool)       // ARRAY[BOOLEAN] (not null)

// Enum types
ColType::Enum("status_enum", vec!["active", "inactive", "suspended"]) // ENUM (not null)
ColType::EnumNull("status_enum", vec!["active", "inactive", "suspended"]) // ENUM (nullable)
ColType::EnumWithDefault("status_enum", vec!["active", "inactive"], "active") // ENUM with default
ColType::EnumNullWithDefault("status_enum", vec!["active", "inactive"], "active") // ENUM nullable with default
```

#### Primary Key Types
```rust
ColType::PkAuto           // Auto-incrementing primary key
ColType::PkUuid           // UUID primary key
```

### Advanced Migration Patterns

#### Enum Management
```rust
// Add new enum values (PostgreSQL only)
add_enum_values(m, "status_enum", vec!["suspended", "cancelled"]).await?;

// Drop enum type completely
drop_enum_type(m, "status_enum").await?;
```

#### Complex Table Creation
```rust
// Create table with multiple references
create_table(m, "orders", &[
    ("order_number", ColType::StringUniq),
    ("total", ColType::DecimalLen(10, 2)),
    ("status", ColType::Enum("order_status", vec!["pending", "confirmed", "shipped"])),
    ("notes", ColType::TextNull),
], &[
    ("customer", ""),           // Creates customer_id
    ("customer", "shipping_address_id"), // Custom column name
    ("customer", "billing_address_id"),   // Custom column name
]).await?;
```

#### Index Creation
```rust
// Create custom indexes
let mut idx = Index::create();
idx.name("idx_posts_published_at")
   .table(Alias::new("posts"))
   .col(Alias::new("published_at"));
m.create_index(idx).await?;
```

### Migration Best Practices

1. **Use helper functions**: Always use Loco's schema helpers instead of raw SQL
2. **Handle rollbacks**: Implement proper `down()` methods for all migrations
3. **Test migrations**: Test both `up()` and `down()` methods
4. **Use descriptive names**: Migration names should clearly indicate the change
5. **Batch operations**: Group related changes in single migrations
6. **Database compatibility**: Helpers handle PostgreSQL/SQLite/MySQL differences automatically

### Example: Complete Migration
```rust
use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Create posts table
        create_table(m, "posts", &[
            ("title", ColType::String),
            ("content", ColType::Text),
            ("excerpt", ColType::TextNull),
            ("published_at", ColType::TimestampWithTimeZoneNull),
            ("status", ColType::Enum("post_status", vec!["draft", "published", "archived"])),
            ("view_count", ColType::IntegerWithDefault(0)),
            ("tags", ColType::Array(ArrayColType::String)),
        ], &[
            ("author", ""),      // Creates author_id
            ("author", "category_id"), // Custom column name
        ]).await?;

        // Create join table for many-to-many
        create_join_table(m, "posts_tags", &[
            ("created_at", ColType::TimestampWithTimeZone),
        ], &[
            ("post", ""),
            ("tag", ""),
        ]).await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        // Drop in reverse order
        drop_table(m, "posts_tags").await?;
        drop_table(m, "posts").await?;
        Ok(())
    }
}
```

## Development Workflow

### 1. Project Setup
```bash
# Create new project (interactive)
loco new

# Navigate to project
cd myapp

# Install dependencies
cargo build
```

**Project Structure Created:**
- `src/` - Contains controllers, models, views, tasks, workers, mailers
- `app.rs` - Main component registration point
- `lib.rs` - Rust-specific exports
- `bin/` - Main binary files
- `controllers/` - HTTP request handlers
- `models/` - Data models and business logic
- `views/` - JSON response structs and templates
- `workers/` - Background job processors
- `mailers/` - Email functionality
- `tasks/` - CLI commands for app management
- `tests/` - App-wide tests
- `config/` - Environment-based configuration
- `migration/` - Database migration files

### 2. Generate Core Components
```bash
# Generate user authentication
cargo loco g scaffold user email:string password:string name:string

# Generate blog posts
cargo loco g scaffold post title:string content:text published:bool author:references

# Generate comments
cargo loco g scaffold comment body:text post:references user:references
```

### 3. Customize Generated Code
- Modify controllers in `src/controllers/`
- Update models in `src/models/`
- Customize views in `src/views/`
- Add business logic to models

### 4. Database Operations
```bash
# Run migrations
cargo loco db migrate

# Generate entities
cargo loco db entities

# Reset database
cargo loco db reset
```

### 5. Development Commands
```bash
# Start the server
cargo loco start

# Start with worker
cargo loco start --worker

# Start server and worker together
cargo loco start --server-and-worker

# Watch and restart (development)
cargo loco watch

# View all routes
cargo loco routes

# Validate app configuration
cargo loco doctor

# Run playground (examples/playground.rs)
cargo playground
```

## Code Patterns

### Controller Structure
```rust
use axum::debug_handler;
use loco_rs::prelude::*;

#[debug_handler]
async fn index() -> Result<Response> {
    // Controller logic
    format::json(data)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/", get(index))
        .add("/:id", get(show))
}
```

### Model Structure
```rust
use async_trait::async_trait;
use loco_rs::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateParams {
    pub title: String,
    pub content: String,
}

impl Model {
    pub async fn create_post(db: &DatabaseConnection, params: CreateParams) -> ModelResult<Self> {
        // Creation logic
    }
}
```

### View Templates
```html
<!-- src/views/posts/index.html -->
{% extends "base.html" %}
{% block content %}
<h1>Posts</h1>
{% for post in posts %}
<div class="post">
    <h2>{{ post.title }}</h2>
    <p>{{ post.content }}</p>
</div>
{% endfor %}
{% endblock %}
```

### JSON Views
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct PostResponse {
    pub id: i32,
    pub title: String,
    pub content: String,
    pub author_name: String,
}

impl PostResponse {
    pub fn new(post: &Post, author: &User) -> Self {
        Self {
            id: post.id,
            title: post.title.clone(),
            content: post.content.clone(),
            author_name: author.name.clone(),
        }
    }
}
```

## Configuration

### Environment Files
- `config/development.yaml`: Development settings
- `config/test.yaml`: Test environment
- `config/production.yaml`: Production settings

**Configuration Structure:**
```yaml
# Database configuration
database:
  uri: "{{ get_env(name='DATABASE_URL', default='sqlite://loco_development.sqlite?mode=rwc') }}"

# Worker configuration
workers:
  mode: BackgroundAsync  # BackgroundQueue, ForegroundBlocking, BackgroundAsync

# Queue configuration (for BackgroundQueue mode)
queue:
  kind: Redis  # Redis, Postgres, Sqlite
  uri: "{{ get_env(name='REDIS_URL', default='redis://127.0.0.1') }}"
  num_workers: 2

# Asset configuration
static:
  enable: true
  folder:
    uri: "/static"
    path: "assets/static"
```

### Database Configuration
```yaml
database:
  url: postgres://user:pass@localhost/dbname
  enable_logging: true
  connect_timeout: 10
  idle_timeout: 300
```

### Background Worker Configuration
```yaml
background:
  mode: redis
  redis:
    url: redis://localhost:6379
```

## Testing

### Test Structure
```rust
use loco_rs::testing;

#[tokio::test]
async fn test_user_creation() {
    let boot = testing::boot_test::<App>().await;
    
    // Test logic
}
```

**Testing Patterns:**
- **Model tests**: `tests/models/` - Test business logic and database operations
- **Request tests**: `tests/requests/` - Test HTTP endpoints and controllers
- **Task tests**: `tests/tasks/` - Test CLI tasks
- **Worker tests**: `tests/workers/` - Test background job processing

### Database Testing
```rust
use loco_rs::testing;

#[tokio::test]
async fn test_with_db() {
    let boot = testing::boot_test::<App>().await;
    
    // Database operations
    let user = users::Model::create_user(&boot.app_context.db, params).await?;
    assert_eq!(user.name, "John");
}
```

## Deployment

### Shuttle Deployment
```bash
# Deploy to Shuttle
cargo shuttle deploy
```

### Docker Deployment
```bash
# Build Docker image
docker build -t my-app .

# Run container
docker run -p 3000:3000 my-app
```

## Best Practices

### 1. Use CLI for Scaffolding
- Always use `cargo loco generate` for new components
- Don't manually create files that can be generated
- Use scaffold for complete CRUD operations

### 2. Follow Naming Conventions
- Controllers: plural nouns (users, posts)
- Models: singular nouns (user, post)
- Routes: RESTful patterns

### 3. Database Design
- Use migrations for schema changes
- Define relationships in models
- Use proper field types and constraints

### 4. Error Handling
- Use `Result<T>` types consistently
- Implement proper validation in models
- Handle database errors gracefully

## Common Commands Reference

```bash
# Project generation
loco new <app-name> [options]

# Local CLI commands
cargo loco generate controller <name> <actions>
cargo loco g model <name> <fields>
cargo loco g scaffold <name> <fields>
cargo loco g migration <name>
cargo loco g task <name>
cargo loco g data <name>

# Database operations
cargo loco db migrate
cargo loco db entities
cargo loco db reset

# Development
cargo loco start
cargo loco start --worker
cargo loco start --server-and-worker
cargo loco watch

# Utilities
cargo loco routes
cargo loco doctor
cargo loco task <task_name> [params]
cargo playground
```

**Get Help**: Run `cargo loco --help` or `cargo loco <command> --help` for detailed command information.

## Task System

Tasks in Loco serve as ad-hoc functionalities for handling specific aspects of your application:

```bash
# Generate a new task
cargo loco g task user_report

# Run a task
cargo loco task user_report

# Run with parameters
cargo loco task user_report var1:val1 var2:val2

# List all tasks
cargo loco task
```

**Task Implementation:**
```rust
use loco_rs::prelude::*;
use loco_rs::task::Vars;

pub struct UserReport;

#[async_trait]
impl Task for UserReport {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "user_report".to_string(),
            detail: "output a user report".to_string(),
        }
    }

    async fn run(&self, app_context: &AppContext, vars: &Vars) -> Result<()> {
        let users = users::Entity::find().all(&app_context.db).await?;
        println!("args: {vars:?}");
        // Task logic here
        Ok(())
    }
}
```

## Background Workers

Loco supports background job processing with multiple backends:

### Worker Modes
- **BackgroundAsync**: In-process async background tasks (default)
- **BackgroundQueue**: Distributed queue processing
- **ForegroundBlocking**: Synchronous blocking tasks

### Queue Backends
- **Redis**: Distributed queue with Redis
- **PostgreSQL**: Database-backed job queue
- **SQLite**: Local job processing

### Configuration
```yaml
workers:
  mode: BackgroundQueue

queue:
  kind: Redis
  uri: "{{ get_env(name='REDIS_URL', default='redis://127.0.0.1') }}"
  num_workers: 2
```

### Running Workers
```bash
# Start standalone worker
cargo loco start --worker

# Start server and worker together
cargo loco start --server-and-worker
```

## Data System

Loco provides static data loaders for configuration and read-only data:

```bash
# Generate a new data loader
cargo loco g data stocks

# Data files are placed in data/ folder
# Access from anywhere in your code
```

**Data Implementation:**
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Stocks {
    pub is_loaded: bool,
    // Your data structure here
}

// Access data anywhere
let data = data::stocks::get();
```

## Authentication

Loco provides JWT-based authentication out of the box with the SaaS starter.

### JWT Authentication
```rust
use loco_rs::auth::JWT;

async fn protected_route(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // Get current user from JWT claims
    let current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Your protected logic here
    Ok(())
}
```

### Built-in Auth Endpoints
- `POST /api/auth/register` - User registration
- `POST /api/auth/login` - User login
- `POST /api/auth/forgot` - Forgot password
- `POST /api/auth/reset` - Reset password
- `POST /api/auth/verify` - Verify account
- `GET /api/auth/current` - Get current user

## Troubleshooting

### Development Tips
- **Auto-reload**: Use `watchexec --notify -r -- cargo loco start` or `bacon run`
- **Database switching**: Seamlessly move between SQLite and PostgreSQL
- **Playground**: Use `examples/playground.rs` for testing models and database operations
- **Route debugging**: Use `cargo loco routes` to see all registered endpoints
- **Configuration validation**: Use `cargo loco doctor` to check app setup

### Common Issues
1. **Migration errors**: Ensure database is running and accessible
2. **Template errors**: Check Tera syntax in view files
3. **Dependency issues**: Run `cargo clean` and rebuild
4. **Database connection**: Verify connection strings in config files

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo loco start
```

This document provides the essential context for an LLM to effectively work with Loco projects, focusing on CLI-driven development and proper code generation patterns. 
