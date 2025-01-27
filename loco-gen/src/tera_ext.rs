use std::collections::HashMap;

use tera::{Tera, Value};

#[must_use]
pub fn new() -> Tera {
    let mut tera = Tera::default();
    tera.register_function("render_form_field", FormField);
    tera
}

const DEFAULT_INPUT_CLASS: &str = "flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-base shadow-sm md:text-sm";
struct FormField;

impl tera::Function for FormField {
    #[allow(clippy::too_many_lines)]
    fn call(&self, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let fname = args
            .get("fname")
            .ok_or_else(|| tera::Error::msg("fname is mandatory"))?
            .as_str()
            .ok_or_else(|| tera::Error::msg("fname must be a string"))?;
        let ftype = args
            .get("ftype")
            .ok_or_else(|| tera::Error::msg("ftype is mandatory"))?
            .as_str()
            .ok_or_else(|| tera::Error::msg("ftype must be a string"))?;
        let rust_type = args
            .get("rust_type")
            .ok_or_else(|| tera::Error::msg("rust_type is mandatory"))?
            .as_str()
            .ok_or_else(|| tera::Error::msg("rust_type must be a string"))?;

        let is_edit_form = args
            .get("edit_form")
            .unwrap_or(&Value::Bool(false))
            .as_bool()
            .unwrap_or_default();

        let value = if is_edit_form {
            format!("{{{{item.{fname}}}}}")
        } else {
            String::new()
        };

        let input_class = args
            .get("input_class")
            .and_then(|c| c.as_str())
            .unwrap_or(DEFAULT_INPUT_CLASS);

        let is_required = ftype.ends_with('!') || ftype.ends_with('^');
        let required_value = if is_required { "required" } else { "" };

        let element = match rust_type {
            "Uuid" | "Option<Uuid>" => {
                let desc = input_description("e.g: 11111111-1111-1111-1111-111111111111.");
                let input = input_string(
                    fname,
                    &value,
                    is_required,
                    input_class,
                    Some(
                        r#"pattern="[0-9A-Fa-f]{{8}}-[0-9A-Fa-f]{{4}}-[0-9A-Fa-f]{{4}}-[0-9A-Fa-f]{{4}}-[0-9A-Fa-f]{{12}}""#,
                    ),
                );
                format!(
                    r"{input}
    {desc}",
                )
            }
            "serde_json::Value" | "Option<serde_json::Value>" => {
                format!(
                    r#"<textarea class="{input_class}" id="{fname}" name="{fname}" type="text" rows="10" cols="50" {required_value}>{value}</textarea>"#,
                )
            }
            "String" | "Option<String>" => {
                input_string(fname, &value, is_required, input_class, None)
            }

            "i8" | "Option<i8>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((i8::MIN, i8::MAX)),
                Some(r#"step="1""#),
            ),
            "i16" | "Option<i16>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((i16::MIN, i16::MAX)),
                Some(r#"step="1""#),
            ),
            "i32" | "Option<i32>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((i32::MIN, i32::MAX)),
                Some(r#"step="1""#),
            ),
            "i64" | "Option<i64>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((i64::MIN, i64::MAX)),
                Some(r#"step="1""#),
            ),
            "Decimal" | "Option<Decimal>" => input_number::<i128>(
                fname,
                &value,
                is_required,
                input_class,
                Some((
                    -79_228_162_514_264_337_593_543_950_335,
                    79_228_162_514_264_337_593_543_950_335,
                )),
                Some(r#"step="0.1""#),
            ),
            "f32" | "Option<f32>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((f32::MIN, f32::MAX)),
                Some(r#"step="0.1""#),
            ),
            "f64" | "Option<f64>" => input_number(
                fname,
                &value,
                is_required,
                input_class,
                Some((f64::MIN, f64::MAX)),
                Some(r#"step="0.1""#),
            ),
            "DateTimeWithTimeZone"
            | "Option<DateTimeWithTimeZone>"
            | "DateTime"
            | "Option<DateTime>"
            | "DateTimeUtc"
            | "Option<DateTimeUtc>" => {
                format!(
                    r#"<input class="{input_class}" id="{fname}" name="{fname}" type="datetime-local" value="{value}" {required_value} />"#,
                )
            }
            "Date" | "Option<Date>" => {
                format!(
                    r#"<input class="{input_class}" id="{fname}" name="{fname}" type="date" value="{value}" {required_value} />"#,
                )
            }
            "bool" | "Option<bool>" => {
                let checked = if is_edit_form {
                    format!("{{% if item.{fname} %}}checked{{%endif %}}")
                } else {
                    String::new()
                };
                format!(
                    r#"<input class="flex rounded-md border border-input bg-transparent text-base shadow-sm md:text-sm" id="{fname}" name="{fname}" type="checkbox" value="true" {checked} {required_value} />"#,
                )
            }
            "Vec<u8>" | "Option<Vec<u8>>" => {
                format!(
                    r#"<input class="{input_class}" id="{fname}" name="{fname}" value="{value}" custom_type="blob" pattern="^[0-9]+(,[0-9]+)*$" {required_value} />
    <p id=":rh:-form-item-description" class="text-[0.8rem] text-muted-foreground">e.g: 123,123,123 .</p>"#,
                )
            }
            "Vec<String>" | "Option<Vec<String>>" => {
                format!(
                    r#"<button type="button" class="text-xs py-1 px-3 rounded-lg bg-gray-900 text-white add-more" data-group="{fname}">Add More</button>
    <div id="{fname}-inputs" class="space-y-2">
    {{% if item.{fname} %}}
        {{% for val in item.{fname} %}}
            <input class="{input_class}" name="{fname}" type="text" value="{{{{val}}}}" {required_value} custom_type="array"/>
        {{% endfor -%}}
    {{%- else -%}}
        <input class="{input_class}" name="{fname}" type="text" value="{value}" {required_value} custom_type="array"/>
    {{%- endif -%}}
    </div>"#
                )
            }
            "Vec<f32>" | "Option<Vec<f32>>" => {
                let edit_input = input_number(
                    fname,
                    "{{val}}",
                    is_required,
                    input_class,
                    Some((f32::MIN, f32::MAX)),
                    Some(r#"custom_type="array" step="0.1""#),
                );
                let create_input = input_number(
                    fname,
                    &value,
                    is_required,
                    input_class,
                    Some((f32::MIN, f32::MAX)),
                    Some(r#"custom_type="array" step="0.1""#),
                );
                input_group(fname, &create_input, &edit_input)
            }
            "Vec<f64>" | "Option<Vec<f64>>" => {
                let edit_input = input_number(
                    fname,
                    "{{val}}",
                    is_required,
                    input_class,
                    Some((f64::MIN, f64::MAX)),
                    Some(r#"custom_type="array""#),
                );
                let create_input = input_number(
                    fname,
                    &value,
                    is_required,
                    input_class,
                    Some((f64::MIN, f64::MAX)),
                    Some(r#"custom_type="array""#),
                );
                input_group(fname, &create_input, &edit_input)
            }
            "Vec<i32>" | "Option<Vec<i32>>" => {
                let edit_input = input_number(
                    fname,
                    "{{val}}",
                    is_required,
                    input_class,
                    Some((i32::MIN, i32::MAX)),
                    Some(r#"custom_type="array""#),
                );
                let create_input = input_number(
                    fname,
                    &value,
                    is_required,
                    input_class,
                    Some((i32::MIN, i32::MAX)),
                    Some(r#"custom_type="array""#),
                );
                input_group(fname, &create_input, &edit_input)
            }
            "Vec<bool>" | "Option<Vec<bool>>" => String::new(),
            _ => {
                return Err(tera::Error::msg(format!(
                    "rust_type: `{rust_type}` not implemented"
                )))
            }
        };

        Ok(Value::String(format!(
            r#"<div class="space-y-2">
    <label class="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70" for=":r2l:-form-item">{fname}</label>
    {element}
</div>"#
        )))
    }

    fn is_safe(&self) -> bool {
        true
    }
}

fn input_group(fname: &str, create_input: &str, edit_input: &str) -> String {
    format!(
        r#"<button type="button" class="text-xs py-1 px-3 rounded-lg bg-gray-900 text-white add-more" data-group="{fname}">Add More</button>
    <div id="{fname}-inputs" class="space-y-2">
    {{% if item.{fname} %}}
        {{% for val in item.{fname} %}}
            {edit_input}
        {{% endfor -%}}
    {{%- else -%}}
        {create_input}
    {{%- endif -%}}
    </div>"#
    )
}

fn input_string(
    name: &str,
    value: &str,
    is_required: bool,
    class: &str,
    attr: Option<&str>,
) -> String {
    let attr = attr.unwrap_or_default();
    let required_value = if is_required { "required" } else { "" };
    format!(
        r#"<input class="{class}" id="{name}" name="{name}" type="text" value="{value}" {required_value} {attr}/>"#
    )
}

fn input_number<T>(
    name: &str,
    value: &str,
    is_required: bool,
    class: &str,
    range: Option<(T, T)>,
    attr: Option<&str>,
) -> String
where
    T: PartialOrd + std::fmt::Display,
{
    let required_value = if is_required { "required" } else { "" };

    let (min_attr, max_attr) = if let Some((min, max)) = range {
        (format!(r#"min="{min}""#), format!(r#"max="{max}""#))
    } else {
        (String::new(), String::new())
    };

    let attr = attr.unwrap_or_default();
    format!(
        r#"<input class="{class}" {min_attr} {max_attr} id="{name}" name="{name}" type="number" value="{value}" {required_value} {attr} />"#,
    )
}

fn input_description<S: AsRef<str>>(description: S) -> String {
    format!(
        r#"<p id=":rh:-form-item-description" class="text-[0.8rem] text-muted-foreground">{}.</p>"#,
        description.as_ref()
    )
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use crate::get_mappings;
    use insta::assert_snapshot;

    #[test]
    fn can_render_form_field() {
        let mapping = get_mappings();

        let mut template_engine = new();

        template_engine
                .add_raw_template(
                    "template",
                    r"{{ render_form_field(fname=fname_val, ftype=ftype_val, rust_type=rust_type_val, edit_form=edit_form_val)}}"
                )
                .unwrap_or_else(|_| panic!("Failed to add raw template"));

        for field in &mapping.field_types {
            let rust_fields = match &field.rust {
                crate::RustType::String(rust_field) => {
                    HashMap::from([(field.name.to_string(), rust_field.to_string())])
                }
                crate::RustType::Map(data) => data.clone(),
            };

            for (field_name, rust_field_type) in rust_fields {
                let mut template_ctx = tera::Context::new();
                template_ctx.insert("fname_val", &field.name);
                template_ctx.insert("ftype_val", &field.name);
                template_ctx.insert("rust_type_val", &rust_field_type);
                template_ctx.insert("edit_form_val", &false);

                let create_form = template_engine
                    .render("template", &template_ctx)
                    .unwrap_or_else(|err| {
                        panic!("Failed to render template. context: {template_ctx:?} .err: {err:?}")
                    });

                template_ctx.insert("edit_form_val", &true);
                let edit_form = template_engine
                    .render("template", &template_ctx)
                    .unwrap_or_else(|err| {
                        panic!("Failed to render template. context: {template_ctx:?} .err: {err:?}")
                    });

                assert_snapshot!(
                    format!("can_render_form_field_[form_{}_{}]", field.name, field_name),
                    format!("Crete form\n\r{create_form}\n\rEdit Form\n\r{edit_form}")
                );
            }
        }
    }
}
