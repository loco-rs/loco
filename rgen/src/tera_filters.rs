use std::{collections::HashMap, hash::BuildHasher};

use heck::{ToKebabCase, ToLowerCamelCase, ToSnakeCase, ToTitleCase, ToUpperCamelCase};
use tera::{to_value, try_get_value, Result, Tera, Value};

/// Registers all available filters for a given `Tera` instance.
pub fn register_all(tera: &mut Tera) {
    tera.register_filter("pascal_case", pascal_case);
    tera.register_filter("camel_case", camel_case);
    tera.register_filter("kebab_case", kebab_case);
    tera.register_filter("lower_case", lower_case);
    tera.register_filter("snake_case", snake_case);
    tera.register_filter("title_case", title_case);
    tera.register_filter("upper_case", upper_case);
}

/// Converts text into `PascalCase`.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::camel_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "some text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("pascal_case", pascal_case);
///
/// let i = "{{ i | camel_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "SomeText");
/// ```
pub fn pascal_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("camel_case", "value", String, value);
    Ok(to_value(s.to_upper_camel_case()).unwrap())
}

/// Converts text into camelCase.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::camel_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "some text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("camel_case", camel_case);
///
/// let i = "{{ i | camel_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "someText");
/// ```
pub fn camel_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("camel_case", "value", String, value);
    Ok(to_value(s.to_lower_camel_case()).unwrap())
}

/// Converts text into kebab-case.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::kebab_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "some text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("kebab_case", kebab_case);
///
/// let i = "{{ i | kebab_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "some-text");
/// ```
pub fn kebab_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("kebab_case", "value", String, value);
    Ok(to_value(s.to_kebab_case()).unwrap())
}

/// Converts text into lowercase.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::lower_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "soMe Text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("lower_case", lower_case);
///
/// let i = "{{ i | lower_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "some text");
/// ```
pub fn lower_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("lower_case", "value", String, value);
    Ok(to_value(s.to_lowercase()).unwrap())
}

/// Converts text into `snake_case`.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::snake_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "soMe Text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("snake_case", snake_case);
///
/// let i = "{{ i | snake_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "some_text");
/// ```
pub fn snake_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("snake_case", "value", String, value);
    Ok(to_value(s.to_snake_case()).unwrap())
}

/// Converts text into Title Case.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::title_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "soMe Text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("title_case", title_case);
///
/// let i = "{{ i | title_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "some_text");
/// ```
pub fn title_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("title_case", "value", String, value);
    Ok(to_value(s.to_title_case()).unwrap())
}

/// Converts text into UPPERCASE.
///
/// # Example
///
/// ```ignore
/// use tera::{Context, Tera};
/// use tera_text_filters::upper_case;
///
/// let mut ctx = Context::new();
/// ctx.insert("i", "soMe Text");
///
/// let mut tera = Tera::default();
/// tera.register_filter("upper_case", upper_case);
///
/// let i = "{{ i | upper_case }}";
/// let rendered = tera.render_str(i, &ctx).unwrap();
/// assert_eq!(rendered, "SOME TEXT");
/// ```
pub fn upper_case<S: BuildHasher>(value: &Value, _: &HashMap<String, Value, S>) -> Result<Value> {
    let s = try_get_value!("upper_case", "value", String, value);
    Ok(to_value(s.to_uppercase()).unwrap())
}
