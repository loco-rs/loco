pub mod number;
pub mod text;
pub mod time;

pub fn register_filters(tera: &mut tera::Tera) {
    tera.register_filter("number_with_delimiter", number::number_with_delimiter);
    tera.register_filter("number_to_human_size", number::number_to_human_size);
    tera.register_filter("number_to_percentage", number::number_to_percentage);
    tera.register_filter("number_to_human", number::number_to_human);
    // Replaces Tera's built-in `pluralize`, keeping its behaviour when no
    // `word` is given.
    tera.register_filter("pluralize", text::pluralize);
    tera.register_filter("to_sentence", text::to_sentence);
    tera.register_filter("ordinalize", text::ordinalize);
    tera.register_filter("time_ago_in_words", time::time_ago_in_words);
    tera.register_filter("distance_of_time_in_words", time::distance_of_time_in_words);
}
