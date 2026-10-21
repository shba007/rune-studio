#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Stop,
    StopListening,
    Yes,
    No,
    Say(String),
}
