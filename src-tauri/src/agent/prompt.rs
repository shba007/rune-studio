const HISTORY_WINDOW: usize = 12;

pub fn plan_user_message(task: &str) -> String {
    format!(
        "Task: {task}\n\nRecording capabilities: {}",
        crate::recording::MODEL_CONTRACT
    )
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub action: String,
    pub outcome: String,
}

pub fn history_lines(history: &[Entry]) -> Vec<String> {
    let mut lines = Vec::new();
    let skipped = history.len().saturating_sub(HISTORY_WINDOW);
    if skipped > 0 {
        let failed = history[..skipped]
            .iter()
            .filter(|entry| entry.outcome != "done")
            .count();
        lines.push(format!(
            "(... {skipped} earlier steps omitted, {failed} of them did not work)"
        ));
    }
    for (index, entry) in history.iter().enumerate().skip(skipped) {
        lines.push(format!(
            "{}. {} -> {}",
            index + 1,
            entry.action,
            entry.outcome
        ));
    }
    lines
}

#[allow(clippy::too_many_arguments)]
pub fn user_message(
    task: &str,
    plan: &[String],
    current: usize,
    history: &[Entry],
    elements: &str,
    warning: Option<&str>,
    step: u32,
    max_steps: u32,
) -> String {
    let mut text = format!(
        "Task: {task}\n\nRecording capabilities: {}\n\n",
        crate::recording::MODEL_CONTRACT
    );
    if !plan.is_empty() {
        text.push_str("Plan:\n");
        for (index, goal) in plan.iter().enumerate() {
            let mark = match index.cmp(&current) {
                std::cmp::Ordering::Less => "done",
                std::cmp::Ordering::Equal => "now",
                std::cmp::Ordering::Greater => "later",
            };
            text.push_str(&format!("{}. {goal} ({mark})\n", index + 1));
        }
        text.push('\n');
    }
    if history.is_empty() {
        text.push_str("No actions yet.\n");
    } else {
        text.push_str("Actions so far:\n");
        for line in history_lines(history) {
            text.push_str(&line);
            text.push('\n');
        }
    }
    if !elements.trim().is_empty() {
        text.push_str(&format!(
            "\nControls in the front window (ids for click_element):\n{}\n",
            elements.trim()
        ));
    }
    if let Some(warning) = warning {
        text.push_str(&format!("\nWARNING: {warning}\n"));
    }
    text.push_str(&format!(
        "\nStep {step} of at most {max_steps}. The screenshot is the screen now. Call one tool."
    ));
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: usize) -> Entry {
        Entry {
            action: format!("click {n}"),
            outcome: "ok".into(),
        }
    }

    #[test]
    fn planning_and_decisions_explain_recording_actions_and_lifetimes() {
        for text in [
            plan_user_message("record while typing"),
            user_message("record while typing", &[], 0, &[], "", None, 1, 30),
        ] {
            assert!(text.contains("start_recording"));
            assert!(text.contains("stop_recording"));
            assert!(text.contains("first captured frame"));
            assert!(text.contains("survives task boundaries"));
            assert!(!text.contains("There are no separate model start/stop"));
        }
    }

    #[test]
    fn first_turn_says_there_is_no_history() {
        let text = user_message("open notepad", &[], 0, &[], "", None, 1, 30);
        assert!(text.contains("Task: open notepad"));
        assert!(text.contains("No actions yet"));
        assert!(text.contains("Step 1 of at most 30"));
        assert!(!text.contains("Plan:"));
    }

    #[test]
    fn controls_and_warnings_are_added_only_when_there_are_some() {
        let text = user_message(
            "t",
            &[],
            0,
            &[],
            "e1 button \"Save\"\ne2 edit \"Name\"",
            Some("stuck"),
            3,
            30,
        );
        assert!(text.contains("e1 button \"Save\""));
        assert!(text.contains("WARNING: stuck"));
        let plain = user_message("t", &[], 0, &[], "  ", None, 3, 30);
        assert!(!plain.contains("Controls in the front window"));
        assert!(!plain.contains("WARNING"));
    }

    #[test]
    fn the_plan_marks_done_now_and_later() {
        let plan = vec![
            "Open Notepad".to_string(),
            "Type hello".to_string(),
            "Save".to_string(),
        ];
        let text = user_message("t", &plan, 1, &[], "", None, 2, 30);
        assert!(text.contains("1. Open Notepad (done)"));
        assert!(text.contains("2. Type hello (now)"));
        assert!(text.contains("3. Save (later)"));
    }

    #[test]
    fn history_is_numbered_and_old_steps_are_dropped() {
        let history: Vec<Entry> = (1..=20).map(entry).collect();
        let text = user_message("t", &[], 0, &history, "", None, 21, 30);
        assert!(text.contains("8 earlier steps omitted"));
        assert!(text.contains("20. click 20 -> ok"));
        assert!(!text.contains("\n3. click 3"));
    }
}
