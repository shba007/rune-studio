#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Stop,
    StopListening,
    Yes,
    No,
    Say(String),
}

pub fn from_transcript(text: &str) -> Option<Intent> {
    let original = text.trim();
    if original.is_empty() || super::direct::repeats_a_word(original) {
        return None;
    }
    let normalized = original
        .trim_matches(|c: char| c.is_whitespace() || matches!(c, '.' | ',' | '!' | '?' | '…'))
        .to_lowercase()
        .replace('’', "'");
    if normalized.is_empty() {
        return None;
    }
    let intent = match normalized.as_str() {
        "stop" | "cancel" | "cancel that" | "never mind" | "nevermind" => Intent::Stop,
        "stop listening" | "mute" | "that's all" | "thats all" => Intent::StopListening,
        "yes" | "go ahead" | "do it" => Intent::Yes,
        "no" | "don't" | "dont" | "deny" => Intent::No,
        _ => Intent::Say(original.into()),
    };
    Some(intent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_phrases_must_be_complete_utterances() {
        for (input, expected) in [
            ("Stop!", Intent::Stop),
            ("cancel that.", Intent::Stop),
            ("stop listening", Intent::StopListening),
            ("That’s all.", Intent::StopListening),
            ("Go ahead", Intent::Yes),
            ("don't!", Intent::No),
            ("stop the music", Intent::Say("stop the music".into())),
            (
                "yes please open the app",
                Intent::Say("yes please open the app".into()),
            ),
        ] {
            assert_eq!(from_transcript(input), Some(expected));
        }
        assert_eq!(from_transcript("..."), None);
        assert_eq!(from_transcript("hello hello hello hello"), None);
    }
}
