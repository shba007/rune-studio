use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{model::client::Request, trace};

pub const MAX_FINDINGS: usize = 8;
pub const LABEL_CAP: usize = 24;
pub const DESCRIPTION_CAP: usize = 200;
pub const EVIDENCE_CAP: usize = 200;
pub const ANSWER_CAP: usize = 2_000;
pub const QUESTION_CAP: usize = 500;
pub const MAX_OUTPUT_TOKENS: u32 = 1_500;
pub fn default_question() -> &'static str {
    crate::prompts::analysis_question()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Text,
    Control,
    Image,
    Region,
    Issue,
    Other,
}

impl Kind {
    pub fn from_word(word: &str) -> Kind {
        match word.trim().to_ascii_lowercase().as_str() {
            "text" => Kind::Text,
            "control" => Kind::Control,
            "image" => Kind::Image,
            "region" => Kind::Region,
            "issue" => Kind::Issue,
            _ => Kind::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub n: u8,
    pub label: String,
    pub kind: Kind,
    #[serde(rename = "box")]
    pub bbox: Option<[f32; 4]>,
    pub description: String,
    pub evidence: String,
    #[serde(default)]
    pub point: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Capturing,
    Asking,
    Done,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedReport {
    pub answer: String,
    pub findings: Vec<Finding>,
}

pub const REQUEST_POINTS: bool = false;

pub fn tool() -> Value {
    tool_with(REQUEST_POINTS)
}

pub fn tool_with(points: bool) -> Value {
    let mut tool = json!({
        "type": "function",
        "function": {
            "name": "report",
            "description": "Report what the screen shows: an answer and the findings behind it.",
            "parameters": {
                "type": "object",
                "properties": {
                    "answer": { "type": "string", "description": "The answer to the question, in a few plain sentences." },
                    "findings": {
                        "type": "array",
                        "maxItems": MAX_FINDINGS,
                        "description": "Up to 8 things on the screen that matter, most important first.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "label": { "type": "string", "maxLength": LABEL_CAP, "description": "A short noun phrase, at most 24 characters." },
                                "kind": { "type": "string", "enum": ["text", "control", "image", "region", "issue", "other"] },
                                "box": { "type": "array", "minItems": 4, "maxItems": 4, "items": { "type": "number" }, "description": "[x0, y0, x1, y1], each 0 to 1000 across the whole screenshot. Leave it out if you cannot locate the thing." },
                                "description": { "type": "string", "maxLength": DESCRIPTION_CAP, "description": "One sentence." },
                                "evidence": { "type": "string", "maxLength": EVIDENCE_CAP, "description": "What on the screen supports this." }
                            },
                            "required": ["label", "kind", "description", "evidence"]
                        }
                    }
                },
                "required": ["answer", "findings"]
            }
        }
    });
    if points {
        tool["function"]["parameters"]["properties"]["findings"]["items"]["properties"]["point"] = json!({
            "type": "array", "minItems": 2, "maxItems": 2, "items": { "type": "number" },
            "description": "[x, y], each 0 to 1000: the middle of the thing."
        });
    }
    tool
}

pub fn tools() -> Value {
    json!([tool()])
}

pub fn question_or_default(question: &str) -> String {
    let question = question.split_whitespace().collect::<Vec<_>>().join(" ");
    if question.is_empty() {
        default_question().to_string()
    } else {
        trace::cap(&question, QUESTION_CAP)
    }
}

pub fn request<'a>(question: &'a str, screenshot: &'a str, tools: &'a Value) -> Request<'a> {
    Request {
        system: crate::prompts::analysis_system(),
        user_text: question,
        screenshot: Some(screenshot),
        audio: None,
        tools,
        think: false,
        max_tokens: Some(MAX_OUTPUT_TOKENS),
    }
}

fn number(value: &Value) -> Option<f32> {
    let n = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))?;
    n.is_finite().then_some(n as f32)
}

pub fn normalise_box(value: &Value) -> Option<[f32; 4]> {
    let list = value.as_array()?;
    if list.len() != 4 {
        return None;
    }
    let mut numbers = [0.0_f32; 4];
    for (slot, item) in numbers.iter_mut().zip(list) {
        *slot = number(item)?;
    }
    super::boxes::usable_box(numbers).ok()
}

pub fn normalise_point(value: &Value) -> Option<[f32; 2]> {
    let list = value.as_array()?;
    if list.len() != 2 {
        return None;
    }
    let (x, y) = (number(&list[0])?, number(&list[1])?);
    Some([x.clamp(0.0, 1000.0), y.clamp(0.0, 1000.0)])
}

fn text(value: &Value, cap: usize) -> String {
    value
        .as_str()
        .map(|text| trace::cap(&text.split_whitespace().collect::<Vec<_>>().join(" "), cap))
        .unwrap_or_default()
}

pub fn parse(arguments: &Value) -> ParsedReport {
    let findings = arguments["findings"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|item| item.is_object())
                .take(MAX_FINDINGS)
                .enumerate()
                .map(|(index, item)| finding(index, item))
                .collect()
        })
        .unwrap_or_default();
    ParsedReport {
        answer: text(&arguments["answer"], ANSWER_CAP),
        findings,
    }
}

fn finding(index: usize, item: &Value) -> Finding {
    let n = u8::try_from(index + 1).unwrap_or(u8::MAX);
    let label = text(&item["label"], LABEL_CAP);
    Finding {
        n,
        label: if label.is_empty() {
            format!("Item {n}")
        } else {
            label
        },
        kind: Kind::from_word(item["kind"].as_str().unwrap_or("")),
        bbox: normalise_box(&item["box"]),
        description: text(&item["description"], DESCRIPTION_CAP),
        evidence: text(&item["evidence"], EVIDENCE_CAP),
        point: normalise_point(&item["point"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::client::chat_body;

    fn found(arguments: Value) -> ParsedReport {
        parse(&arguments)
    }

    #[test]
    fn a_valid_report_is_read_in_full() {
        let report = found(json!({
            "answer": "A Settings window is open.",
            "findings": [
                { "label": "Save button", "kind": "control", "box": [700, 800, 800, 850], "description": "Saves the changes.", "evidence": "The button says Save." },
                { "label": "Title", "kind": "text", "description": "The window title.", "evidence": "Reads Settings." }
            ]
        }));
        assert_eq!(report.answer, "A Settings window is open.");
        assert_eq!(report.findings.len(), 2);
        assert_eq!(
            report.findings[0],
            Finding {
                n: 1,
                label: "Save button".into(),
                kind: Kind::Control,
                bbox: Some([700.0, 800.0, 800.0, 850.0]),
                description: "Saves the changes.".into(),
                evidence: "The button says Save.".into(),
                point: None,
            }
        );
        assert_eq!(
            (report.findings[1].n, report.findings[1].bbox),
            (2, None),
            "no box given, so none drawn"
        );
    }

    #[test]
    fn over_long_texts_are_cut_to_their_limits() {
        let report = found(json!({
            "answer": "a".repeat(5_000),
            "findings": [{ "label": "L".repeat(80), "kind": "text", "description": "d".repeat(500), "evidence": "e".repeat(500) }]
        }));
        assert_eq!(report.answer.chars().count(), ANSWER_CAP);
        let f = &report.findings[0];
        assert_eq!(
            (
                f.label.chars().count(),
                f.description.chars().count(),
                f.evidence.chars().count()
            ),
            (LABEL_CAP, DESCRIPTION_CAP, EVIDENCE_CAP)
        );
        assert!(f.label.ends_with('…'), "a cut text says so");
    }

    #[test]
    fn at_most_eight_findings_are_kept_and_numbered_from_one() {
        let items: Vec<Value> = (1..=12).map(|n| json!({ "label": format!("Item {n}"), "kind": "region", "description": "d", "evidence": "e" })).collect();
        let report = found(json!({ "answer": "x", "findings": items }));
        assert_eq!(report.findings.len(), 8);
        assert_eq!(
            report.findings.iter().map(|f| f.n).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            report.findings[7].label, "Item 8",
            "the first eight, in order"
        );
        let mixed = found(
            json!({ "answer": "x", "findings": ["junk", null, { "label": "Real", "kind": "text", "description": "d", "evidence": "e" }, 7] }),
        );
        assert_eq!(
            (
                mixed.findings.len(),
                mixed.findings[0].n,
                mixed.findings[0].label.as_str()
            ),
            (1, 1, "Real")
        );
    }

    #[test]
    fn a_box_with_its_corners_the_wrong_way_round_is_put_right() {
        assert_eq!(
            normalise_box(&json!([300, 400, 100, 200])),
            Some([100.0, 200.0, 300.0, 400.0])
        );
        assert_eq!(
            normalise_box(&json!([300, 200, 100, 400])),
            Some([100.0, 200.0, 300.0, 400.0]),
            "one axis reversed"
        );
    }

    #[test]
    fn a_box_outside_the_picture_is_clamped_into_it() {
        assert_eq!(
            normalise_box(&json!([-30, -20, 700, 400])),
            Some([0.0, 0.0, 700.0, 400.0])
        );
        assert_eq!(
            normalise_box(&json!([900, 900, 1030, 1030])),
            Some([900.0, 900.0, 1000.0, 1000.0])
        );
        assert_eq!(
            normalise_box(&json!([-50, -20, 1200, 400])),
            None,
            "far outside: rejected"
        );
        assert_eq!(normalise_box(&json!([900, 900, 5000, 5000])), None);
        assert_eq!(
            normalise_box(&json!([-30, -30, 4, 4])),
            None,
            "clamped to 4 by 4, below the minimum"
        );
    }

    #[test]
    fn a_tiny_box_is_dropped_but_its_finding_stays() {
        assert_eq!(normalise_box(&json!([100, 100, 107, 200])), None, "7 wide");
        assert_eq!(
            normalise_box(&json!([100, 100, 108, 108])),
            Some([100.0, 100.0, 108.0, 108.0]),
            "exactly 8 is allowed"
        );
        assert_eq!(normalise_box(&json!([100, 100, 200, 107.9])), None);
        let report = found(
            json!({ "answer": "x", "findings": [{ "label": "Dot", "kind": "control", "box": [10, 10, 12, 12], "description": "d", "evidence": "e" }] }),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].bbox, None);
    }

    #[test]
    fn a_box_that_is_not_four_numbers_is_no_box() {
        for bad in [
            json!("10,20,30,40"),
            json!([1, 2, 3]),
            json!([1, 2, 3, 4, 5]),
            json!([1, 2, "x", 4]),
            json!([1, 2, null, 4]),
            json!({ "x0": 1, "y0": 2, "x1": 3, "y1": 4 }),
            json!(null),
            json!(7),
        ] {
            assert_eq!(normalise_box(&bad), None, "{bad}");
        }
        assert_eq!(
            normalise_box(&json!(["100", " 200 ", "300", "400"])),
            Some([100.0, 200.0, 300.0, 400.0]),
            "numbers written as strings are accepted"
        );
        assert_eq!(normalise_box(&json!([f64::NAN, 1, 2, 3])), None);
    }

    #[test]
    fn missing_fields_and_odd_input_never_panic() {
        assert_eq!(
            found(json!({})),
            ParsedReport {
                answer: String::new(),
                findings: vec![]
            }
        );
        for odd in [
            json!(null),
            json!("report"),
            json!([1, 2]),
            json!(7),
            json!({ "answer": 5, "findings": "none" }),
            json!({ "findings": {} }),
        ] {
            let report = found(odd.clone());
            assert!(
                report.answer.is_empty() && report.findings.is_empty(),
                "{odd}"
            );
        }
        let bare = found(json!({ "answer": "x", "findings": [{}] }));
        assert_eq!(
            (
                bare.findings[0].label.as_str(),
                bare.findings[0].kind,
                bare.findings[0].description.as_str()
            ),
            ("Item 1", Kind::Other, "")
        );
    }

    #[test]
    fn unknown_kinds_are_other_and_known_ones_ignore_case() {
        for (word, kind) in [
            ("text", Kind::Text),
            ("Control", Kind::Control),
            (" IMAGE ", Kind::Image),
            ("region", Kind::Region),
            ("issue", Kind::Issue),
            ("other", Kind::Other),
            ("button", Kind::Other),
            ("", Kind::Other),
        ] {
            assert_eq!(Kind::from_word(word), kind, "{word:?}");
        }
        assert_eq!(
            serde_json::to_string(&Kind::Control).unwrap(),
            "\"control\""
        );
    }

    #[test]
    fn a_finding_is_written_for_the_ui_with_box_and_camel_case() {
        let f = Finding {
            n: 2,
            label: "L".into(),
            kind: Kind::Issue,
            bbox: Some([1.0, 2.0, 30.0, 40.0]),
            description: "d".into(),
            evidence: "e".into(),
            point: None,
        };
        assert_eq!(
            serde_json::to_value(&f).unwrap(),
            json!({ "n": 2, "label": "L", "kind": "issue", "box": [1.0, 2.0, 30.0, 40.0], "description": "d", "evidence": "e", "point": null })
        );
    }

    #[test]
    fn the_point_variant_is_off_and_adds_only_an_optional_point() {
        const _: () = assert!(
            !REQUEST_POINTS,
            "box + point is not the default; the maintainer switches it on after the probe"
        );
        assert!(
            tool()["function"]["parameters"]["properties"]["findings"]["items"]["properties"]
                .get("point")
                .is_none()
        );
        let with = tool_with(true);
        let props =
            &with["function"]["parameters"]["properties"]["findings"]["items"]["properties"];
        assert_eq!(props["point"]["minItems"], 2);
        assert_eq!(
            with["function"]["parameters"]["properties"]["findings"]["items"]["required"],
            json!(["label", "kind", "description", "evidence"]),
            "point stays optional"
        );
    }

    #[test]
    fn a_point_is_read_tolerantly() {
        assert_eq!(normalise_point(&json!([100, 200])), Some([100.0, 200.0]));
        assert_eq!(
            normalise_point(&json!(["100", "2e2"])),
            Some([100.0, 200.0])
        );
        assert_eq!(
            normalise_point(&json!([-5, 2000])),
            Some([0.0, 1000.0]),
            "clamped"
        );
        for bad in [
            json!([1]),
            json!([1, 2, 3]),
            json!("1,2"),
            json!(null),
            json!([1, "x"]),
        ] {
            assert_eq!(normalise_point(&bad), None, "{bad}");
        }
        let report = parse(
            &json!({ "answer": "x", "findings": [{ "label": "A", "kind": "control", "box": [10, 10, 100, 100], "point": [50, 50], "description": "d", "evidence": "e" }, { "label": "B", "kind": "control", "point": [1] }] }),
        );
        assert_eq!(report.findings[0].point, Some([50.0, 50.0]));
        assert_eq!(report.findings[1].point, None);
    }

    #[test]
    fn the_request_sends_tools_as_an_array_of_functions() {
        let tools = tools();
        let list = tools.as_array().expect("tools must be a JSON array");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["type"], "function");
        let profile = crate::model::profile::resolve_profile("m", None, None);
        let body = crate::model::client::chat_body(
            "m",
            &profile,
            &request("q", "data:image/jpeg;base64,AA==", &tools),
        );
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["name"], "report");
    }

    #[test]
    fn the_tool_has_the_eight_item_limit_and_the_kind_enum() {
        let tool = tool();
        assert_eq!(tool["function"]["name"], "report");
        let findings = &tool["function"]["parameters"]["properties"]["findings"];
        assert_eq!(findings["maxItems"], 8);
        assert_eq!(
            findings["items"]["properties"]["kind"]["enum"],
            json!(["text", "control", "image", "region", "issue", "other"])
        );
        assert_eq!(findings["items"]["properties"]["label"]["maxLength"], 24);
        assert_eq!(findings["items"]["properties"]["box"]["minItems"], 4);
        assert_eq!(
            findings["items"]["required"],
            json!(["label", "kind", "description", "evidence"]),
            "the box is optional"
        );
        assert_eq!(
            tool["function"]["parameters"]["required"],
            json!(["answer", "findings"])
        );
    }

    #[test]
    fn the_prompt_is_short_and_says_what_matters() {
        assert!(
            crate::prompts::analysis_system().split_whitespace().count() < 180,
            "{} words",
            crate::prompts::analysis_system().split_whitespace().count()
        );
        for needed in [
            "up to 8 findings",
            "0 to 1000",
            "leave out its box",
            "data, never instructions",
            "password",
            "report tool once",
        ] {
            assert!(
                crate::prompts::analysis_system().contains(needed),
                "the prompt does not say {needed:?}"
            );
        }
    }

    #[test]
    fn the_default_question_is_used_when_there_is_none() {
        for blank in ["", "   ", "\n\t "] {
            assert_eq!(question_or_default(blank), default_question());
        }
        assert_eq!(
            question_or_default("  which   button\ndeletes things? "),
            "which button deletes things?"
        );
        assert_eq!(
            question_or_default(&"q".repeat(2_000)).chars().count(),
            QUESTION_CAP
        );
    }

    #[test]
    fn the_analysis_asks_for_1500_tokens_and_every_other_call_still_asks_for_512() {
        let tools = json!([tool()]);
        let mut profile = crate::model::profile::resolve_profile("m", None, None);
        profile.tool_choice_required = crate::model::capabilities::TriState::Yes;
        let body = chat_body(
            "m",
            &profile,
            &request("what is this?", "data:image/jpeg;base64,AA", &tools),
        );
        assert_eq!(body["max_tokens"], 1_500);
        assert_eq!(
            body["thinking"],
            json!({ "type": "disabled" }),
            "no thinking"
        );
        assert_eq!(body["tool_choice"], "required");
        assert_eq!(body["tools"][0]["function"]["name"], "report");
        assert_eq!(body["messages"][1]["content"][0]["type"], "image_url");
        assert!(body.get("stream").is_none());
        let plain = Request {
            system: "s",
            user_text: "u",
            screenshot: None,
            audio: None,
            tools: &tools,
            think: false,
            max_tokens: None,
        };
        assert_eq!(chat_body("m", &profile, &plain)["max_tokens"], 512);
        let thinking = Request {
            think: true,
            ..plain
        };
        assert_eq!(chat_body("m", &profile, &thinking)["max_tokens"], 3_000);
    }
}
