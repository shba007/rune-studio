use std::sync::OnceLock;

macro_rules! prompt {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/prompts/", $path)).trim()
    };
}

pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    values
        .iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

pub fn agent_system() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        fill(
            prompt!("agent/system.md"),
            &[
                ("controls", prompt!("skills/controls.md")),
                ("typing", prompt!("skills/typing.md")),
                ("persistence", prompt!("skills/persistence.md")),
            ],
        )
    })
}

pub fn agent_plan() -> &'static str {
    prompt!("agent/plan.md")
}

pub fn agent_stuck(unchanged: u32) -> String {
    fill(
        prompt!("agent/stuck.md"),
        &[("unchanged", &unchanged.to_string())],
    )
}

pub fn agent_contradicted(evidence: &str) -> String {
    fill(prompt!("agent/contradicted.md"), &[("evidence", evidence)])
}

pub fn verify_system() -> &'static str {
    prompt!("verify/system.md")
}

pub fn voice_direct() -> &'static str {
    prompt!("voice/direct.md")
}

pub fn analysis_system() -> &'static str {
    prompt!("analysis/system.md")
}

pub fn analysis_point() -> &'static str {
    prompt!("analysis/point.md")
}

pub fn analysis_question() -> &'static str {
    prompt!("analysis/question.md")
}

pub fn quick_simple() -> &'static str {
    prompt!("quick/simple.md")
}

pub fn quick_finished() -> &'static str {
    prompt!("quick/finished.md")
}

pub fn quick_risk() -> &'static str {
    prompt!("quick/risk.md")
}

pub fn quick_loading() -> &'static str {
    prompt!("quick/loading.md")
}

pub fn quick_blocker() -> &'static str {
    prompt!("quick/blocker.md")
}

pub fn blocker_hint(blocker: &str) -> Option<&'static str> {
    Some(match blocker {
        "cookie_banner" => prompt!("skills/cookie_banner.md"),
        "popup" => prompt!("skills/popup.md"),
        "login_required" => prompt!("skills/login_required.md"),
        "captcha" => prompt!("skills/captcha.md"),
        "error_page" => prompt!("skills/error_page.md"),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_system_prompt_has_its_skills_filled_in_and_no_slot_left() {
        let text = agent_system();
        assert!(text.contains("click_element"));
        assert!(text.contains("press_keys ctrl+a"));
        assert!(!text.contains("{controls}") && !text.contains("{typing}"));
    }

    #[test]
    fn slots_are_replaced_and_nothing_else_is_touched() {
        assert_eq!(fill("a {x} b {x} {y}", &[("x", "1")]), "a 1 b 1 {y}");
        assert!(agent_stuck(3).contains("last 3 actions"));
        assert!(!agent_stuck(3).contains('{'));
        assert!(
            agent_contradicted("a window")
                .contains("check says the screen does not show the task as done: a window.")
        );
    }

    #[test]
    fn every_prompt_file_is_there_and_not_empty() {
        for text in [
            agent_plan(),
            verify_system(),
            voice_direct(),
            analysis_system(),
            analysis_point(),
            analysis_question(),
            quick_simple(),
            quick_finished(),
            quick_risk(),
            quick_loading(),
            quick_blocker(),
        ] {
            assert!(text.len() > 20);
        }
        for blocker in [
            "cookie_banner",
            "popup",
            "login_required",
            "captcha",
            "error_page",
        ] {
            assert!(blocker_hint(blocker).is_some());
        }
        assert!(blocker_hint("none").is_none());
    }
}
