use std::sync::{Arc, Mutex};

use lazy_static::lazy_static;
use loco_rs::Result;
use rust_bert::pipelines::{
    common::ModelType,
    text_generation::{TextGenerationConfig, TextGenerationModel},
};

lazy_static! {
    pub static ref MODEL: Arc<Mutex<TextGenerationModel>> = {
        let generate_config = TextGenerationConfig {
            model_type: ModelType::GPT2,
            max_length: Some(30),
            do_sample: false,
            num_beams: 1,
            temperature: 1.0,
            num_return_sequences: 1,
            ..Default::default()
        };
        let model = tokio::task::block_in_place(|| {
            TextGenerationModel::new(generate_config).expect("model")
        });

        Arc::new(Mutex::new(model))
    };
}

pub fn load() {
    let _model = MODEL.lock();
}

/// LLM infer
///
/// # Panics
///
/// Panics if cant take a lock or async fails
///
/// # Errors
///
/// This function will return an error
pub async fn infer(input_context: &str) -> Result<String> {
    let input = input_context.to_string();
    let output = tokio::task::spawn(async move {
        let output = MODEL.lock().expect("lock").generate(&[&input], None);
        output
    })
    .await
    .expect("await");
    Ok(output.join(""))
}
