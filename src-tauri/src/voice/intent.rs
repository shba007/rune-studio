//! What a spoken sentence means to the app: stop, an answer to a question, or words for the agent.
//!
//! Only whole short utterances count as the control words ("stop", "yes", "no"). A longer
//! sentence that merely contains one ("stop the music in Spotify") is a request for the agent.
//! Nothing here decides whether something is allowed: that is the safety gate's job.

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

/// Lower case, letters and digits only, single spaces ("Don't!" becomes "dont").
fn normalise(text: &str) -> String {
    let kept: String = text.to_lowercase().chars().filter(|c| c.is_alphanumeric() || c.is_whitespace()).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

const STOP: &[&str] = &["stop", "stop it", "stop that", "stop stop", "stop now", "please stop", "cancel", "cancel that", "abort", "halt", "never mind", "nevermind", "enough"];
const STOP_LISTENING: &[&str] = &["stop listening", "turn off the mic", "turn off the microphone", "mute", "thats all", "that is all", "goodbye"];
const YES: &[&str] = &["yes", "yeah", "yep", "yup", "sure", "ok", "okay", "allow", "allow it", "approve", "approved", "proceed", "go ahead", "do it", "go for it", "please do", "yes please", "yes allow"];
const NO: &[&str] = &["no", "nope", "nah", "dont", "do not", "dont do it", "deny", "decline", "not now", "no thanks", "no dont", "no stop"];

pub fn interpret(text: &str) -> Intent {
    let said = normalise(text);
    // "no stop" is a no (and the gate treats a no as stopping that action) before it is a stop.
    if NO.contains(&said.as_str()) {
        return Intent::No;
    }
    if STOP_LISTENING.contains(&said.as_str()) {
        return Intent::StopListening;
    }
    if STOP.contains(&said.as_str()) {
        return Intent::Stop;
    }
    if YES.contains(&said.as_str()) {
        return Intent::Yes;
    }
    Intent::Say(text.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_control_words_are_recognised_whatever_the_punctuation() {
        assert_eq!(interpret("Stop."), Intent::Stop);
        assert_eq!(interpret("  STOP IT!  "), Intent::Stop);
        assert_eq!(interpret("Never mind."), Intent::Stop);
        assert_eq!(interpret("Yes, please."), Intent::Yes);
        assert_eq!(interpret("Go ahead!"), Intent::Yes);
        assert_eq!(interpret("Don't."), Intent::No);
        assert_eq!(interpret("No thanks"), Intent::No);
        assert_eq!(interpret("Stop listening."), Intent::StopListening);
    }

    #[test]
    fn a_longer_sentence_with_a_control_word_in_it_is_a_request() {
        for text in ["stop the music in Spotify", "yes I would like you to open notepad", "No, open the other folder", "cancel my meeting at three"] {
            assert_eq!(interpret(text), Intent::Say(text.to_string()), "{text}");
        }
    }

    #[test]
    fn ordinary_commands_pass_through_trimmed() {
        assert_eq!(interpret("  open Brave and search for Qwen "), Intent::Say("open Brave and search for Qwen".into()));
    }

    #[test]
    fn no_wins_over_stop_and_yes_is_not_taken_from_a_no() {
        assert_eq!(interpret("no stop"), Intent::No);
        assert_ne!(interpret("no"), Intent::Yes);
    }
}
