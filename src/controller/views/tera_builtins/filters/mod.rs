pub mod number;

pub fn register_filters(tera: &mut tera::Tera) {
    tera.register_filter("number_with_delimiter", number::number_with_delimiter);
    tera.register_filter("number_to_human_size", number::number_to_human_size);
    tera.register_filter("number_to_percentage", number::number_to_percentage);
}
