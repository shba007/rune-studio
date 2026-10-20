//! What a spoken sentence means to the app: stop, an answer to a question, or words for the agent.
//! The omni model decides which (`voice::direct`); nothing here decides whether something is
//! allowed: that is the safety gate's job.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    /// Stop everything now.
    Stop,
    /// Close the microphone.
    StopListening,
    Yes,
    No,
    /// Anything else: a task to start, or words to steer the running one with.
    Say(String),
}
