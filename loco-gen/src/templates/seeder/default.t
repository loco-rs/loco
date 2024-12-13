{% set module_name = name |  snake_case -%}
{% set struct_name = module_name | pascal_case -%}
use loco_rs::prelude::*;

pub struct {{struct_name}}Seeder;

impl Seeder for {{struct_name}}Seeder {
    fn name(&self) -> String {
        "{{struct_name}}Seeder".to_string()
    }

    async fn seed(&self, ctx: &AppContext) -> AppResult<()> {
        todo!()
    }
}
