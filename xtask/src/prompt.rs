/// Prompt confirmation message
///
/// # Errors
/// return an error when could prompt the message or could not parse the input
pub fn confirmation(message: &str) -> eyre::Result<bool> {
    let question = requestty::Question::confirm("confirm")
        .message(message)
        .build();

    let res = requestty::prompt_one(question)?;
    let answer = res
        .as_bool()
        .ok_or_else(|| eyre::eyre!("app selection name is empty"))?;

    Ok(answer)
}
