pub mod generate;

#[derive(Debug)]
pub struct CmdExit {
    pub code: i32,
    pub message: Option<String>,
}
