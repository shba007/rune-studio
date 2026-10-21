#![cfg_attr(not(feature = "agent-bridge"), allow(dead_code))]

use serde::Serialize;

const NAME_MAX: usize = 40;
const DEFAULT_NAME: &str = "External agent";
pub const EXTERNAL: &str = "external";

const KNOWN: [(&str, &str, &[&str]); 10] = [
    ("claude", "Claude", &["claude", "claude code"]),
    (
        "antigravity",
        "Antigravity",
        &["antigravity", "google antigravity"],
    ),
    ("opencode", "OpenCode", &["opencode", "open code"]),
    (
        "chatgpt",
        "ChatGPT",
        &["chatgpt", "chat gpt", "openai", "openai chatgpt"],
    ),
    ("codex", "Codex", &["codex", "codex cli", "openai codex"]),
    (
        "gemini",
        "Gemini",
        &["gemini", "gemini cli", "google gemini"],
    ),
    ("cursor", "Cursor", &["cursor"]),
    ("cline", "Cline", &["cline"]),
    ("copilot", "Copilot", &["copilot", "github copilot"]),
    ("windsurf", "Windsurf", &["windsurf"]),
];

#[cfg(test)]
pub fn all_ids() -> Vec<&'static str> {
    KNOWN
        .iter()
        .map(|(id, _, _)| *id)
        .chain(std::iter::once(EXTERNAL))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentIdentity {
    pub id: String,
    pub name: String,
    pub known: bool,
    pub verified: bool,
    pub declared: String,
}

fn normalise(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with(' ') && !out.is_empty() {
            out.push(' ');
        }
    }
    out.trim_end().to_string()
}

fn clean(text: &str) -> String {
    let kept: String = text
        .chars()
        .filter(|c| !c.is_control() && !matches!(*c, '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'))
        .collect();
    kept.trim().chars().take(NAME_MAX).collect()
}

fn find(normalised: &str) -> Option<(&'static str, &'static str)> {
    KNOWN
        .iter()
        .find(|(_, _, aliases)| aliases.contains(&normalised))
        .map(|(id, name, _)| (*id, *name))
}

fn by_id(id: &str) -> Option<(&'static str, &'static str)> {
    KNOWN
        .iter()
        .find(|(known, _, _)| *known == id)
        .map(|(id, name, _)| (*id, *name))
}

pub fn identify(declared: &str, attested: Option<&str>) -> AgentIdentity {
    let declared_clean = clean(declared);
    if let Some((id, name)) = attested
        .map(|id| normalise(id).replace(' ', ""))
        .and_then(|id| by_id(&id))
    {
        return AgentIdentity {
            id: id.into(),
            name: name.into(),
            known: true,
            verified: true,
            declared: declared_clean,
        };
    }
    if let Some((id, name)) = find(&normalise(&declared_clean)) {
        return AgentIdentity {
            id: id.into(),
            name: name.into(),
            known: true,
            verified: false,
            declared: declared_clean,
        };
    }
    let name = if declared_clean.is_empty() {
        DEFAULT_NAME.to_string()
    } else {
        declared_clean.clone()
    };
    AgentIdentity {
        id: EXTERNAL.into(),
        name,
        known: false,
        verified: false,
        declared: declared_clean,
    }
}

pub const ATTEST_HEADER: &str = "x-rune-agent";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_agents_match_by_exact_alias_whatever_the_case_and_punctuation() {
        for (declared, id) in [
            ("Claude", "claude"),
            ("claude-code", "claude"),
            ("  CLAUDE CODE ", "claude"),
            ("Google Antigravity", "antigravity"),
            ("OpenCode", "opencode"),
            ("open_code", "opencode"),
            ("ChatGPT", "chatgpt"),
            ("Codex CLI", "codex"),
            ("gemini-cli", "gemini"),
        ] {
            let who = identify(declared, None);
            assert_eq!(
                (who.id.as_str(), who.known, who.verified),
                (id, true, false),
                "{declared}"
            );
        }
    }

    #[test]
    fn a_substring_is_not_a_match() {
        for declared in [
            "Not Claude",
            "Claude's helper",
            "my-antigravity-fork",
            "cursorless",
            "",
        ] {
            let who = identify(declared, None);
            assert_eq!(
                (who.id.as_str(), who.known),
                (EXTERNAL, false),
                "{declared}"
            );
        }
    }

    #[test]
    fn an_unknown_agent_keeps_its_own_clean_clipped_name_and_an_empty_one_gets_the_default() {
        assert_eq!(identify("Acme Bot", None).name, "Acme Bot");
        assert_eq!(identify("", None).name, "External agent");
        let long = identify(&"x".repeat(400), None);
        assert_eq!(long.name.chars().count(), 40);
        let nasty = identify("Evil\nBot\u{202E}reversed\u{0007}", None);
        assert_eq!(nasty.name, "EvilBotreversed");
        assert!(!nasty.name.contains('\u{202E}') && !nasty.name.contains('\n'));
    }

    #[test]
    fn the_registration_wins_over_the_declared_name_and_the_declared_one_is_kept() {
        let who = identify("Totally Different", Some("claude"));
        assert_eq!(
            (who.id.as_str(), who.name.as_str(), who.known, who.verified),
            ("claude", "Claude", true, true)
        );
        assert_eq!(who.declared, "Totally Different");
        let both = identify("Claude", Some("Antigravity"));
        assert_eq!(both.id, "antigravity", "attested wins over declared");
    }

    #[test]
    fn an_attested_id_that_is_not_in_the_list_is_ignored() {
        let who = identify("Acme Bot", Some("evil"));
        assert_eq!(
            (who.id.as_str(), who.verified, who.name.as_str()),
            (EXTERNAL, false, "Acme Bot")
        );
        assert!(!identify("Claude", Some("")).verified);
    }

    #[test]
    fn every_id_is_listed_for_the_ui_registry() {
        let ids = all_ids();
        assert!(
            ids.contains(&"claude")
                && ids.contains(&"opencode")
                && ids.contains(&"chatgpt")
                && ids.last() == Some(&"external")
        );
        assert_eq!(ids.len(), 11);
    }
}
