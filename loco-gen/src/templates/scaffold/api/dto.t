to: src/dtos/{{ snake_plural }}.rs
skip_exists: true
message: "DTO `{{ pascal_singular }}Dto` was added successfully."
injections:
- into: src/dtos/mod.rs
  append: true
  content: "pub mod {{ snake_plural }};"
---
{% if prelude_use %}{{ prelude_use }}
{% endif %}use ts_rs::TS;

{% for e in enums %}#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
#[serde(rename_all = "snake_case")]
pub enum {{ e.enum_type }} {
{% for v in e.variants %}    {{ v.variant }},
{% endfor %}}

impl From<String> for {{ e.enum_type }} {
    fn from(value: String) -> Self {
        match value.as_str() {
{% for v in e.match_arms %}            "{{ v.value }}" => Self::{{ v.variant }},
{% endfor %}            _ => Self::{{ e.fallback_variant }},
        }
    }
}

impl {{ e.enum_type }} {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
{% for v in e.variants %}            Self::{{ v.variant }} => "{{ v.value }}",
{% endfor %}        }
    }
}

{% endfor -%}
#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct {{ pascal_singular }}Dto {
    #[ts(type = "number")]
    pub id: i64,
{% for f in fields %}{% if f.ts_override %}    #[ts(type = "{{ f.ts_override }}")]
{% endif %}    pub {{ f.field_name }}: {{ f.rust_type }},
{% endfor -%}{% if with_tz %}    #[ts(type = "string")]
    pub created_at: DateTimeWithTimeZone,
    #[ts(type = "string")]
    pub updated_at: DateTimeWithTimeZone,
{% endif %}}

impl From<crate::models::_entities::{{ snake_plural }}::Model> for {{ pascal_singular }}Dto {
    fn from(m: crate::models::_entities::{{ snake_plural }}::Model) -> Self {
        Self {
            id: m.id,
{% for f in fields %}            {{ f.field_name }}: {{ f.from_expr }},
{% endfor -%}{% if with_tz %}            created_at: m.created_at,
            updated_at: m.updated_at,
{% endif %}        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Create{{ pascal_singular }} {
{% for f in fields %}{% if f.ts_override %}    #[ts(type = "{{ f.ts_override }}")]
{% endif %}    pub {{ f.field_name }}: {{ f.rust_type }},
{% endfor %}}

#[derive(serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/bindings/")]
pub struct Update{{ pascal_singular }} {
{% for f in fields %}{% if f.ts_override %}    #[ts(type = "{{ f.ts_override }}")]
{% endif %}    pub {{ f.field_name }}: {{ f.rust_type }},
{% endfor %}}
