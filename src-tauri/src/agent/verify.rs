use std::time::Duration;

use serde_json::{Value, json};

use super::prompt::{self, Entry};
use crate::{
    model::client::{ChatClient, Request},
    settings::VerifyMode,
    trace::{self, EVIDENCE_CAP},
};

pub const TIMEOUT: Duration = Duration::from_secs(30);

const TASK_CAP: usize = 500;
const GOAL_CAP: usize = 200;
const CLAIM_CAP: usize = 400;
const FRONT_CAP: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictResult {
    Verified,
    Contradicted,
    Unsure,
}

impl VerdictResult {
    pub fn as_str(self) -> &'static str {
        match self {
            VerdictResult::Verified => "verified",
            VerdictResult::Contradicted => "contradicted",
            VerdictResult::Unsure => "unsure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub result: VerdictResult,
    pub evidence: String,
}

pub fn tool() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "verdict",
            "description": "Say whether the screen shows the task is done.",
            "parameters": {
                "type": "object",
                "properties": {
                    "result": { "type": "string", "enum": ["verified", "contradicted", "unsure"], "description": "verified: the screen shows the task is done. contradicted: it shows it is not, or contradicts the claim. unsure: the screen does not let you tell." },
                    "evidence": { "type": "string", "description": "One short sentence naming what you see on the screen." }
                },
                "required": ["result", "evidence"]
            }
        }
    })
}

pub struct VerifyInput<'a> {
    pub task: &'a str,
    pub goals: &'a [String],
    pub history: &'a [Entry],
    pub claim_summary: &'a str,
    pub screenshot: &'a str,
    pub front: &'a str,
}

pub fn front_line() -> String {
    let window = crate::safety::foreground::current();
    if window.title.is_empty() && window.process.is_empty() {
        return String::new();
    }
    format!(
        "Window in front (from the system, not from the screenshot): \"{}\", program {}.",
        trace::cap(window.title.trim(), 120),
        if window.process.is_empty() {
            "unknown"
        } else {
            &window.process
        }
    )
}

pub fn user_message(
    task: &str,
    goals: &[String],
    history: &[Entry],
    claim_summary: &str,
    front: &str,
) -> String {
    let mut text = format!("Task: {}\n\n", trace::cap(task.trim(), TASK_CAP));
    if !goals.is_empty() {
        text.push_str("Plan:\n");
        for (index, goal) in goals.iter().enumerate() {
            text.push_str(&format!(
                "{}. {}\n",
                index + 1,
                trace::cap(goal.trim(), GOAL_CAP)
            ));
        }
        text.push('\n');
    }
    let lines = prompt::history_lines(history);
    if lines.is_empty() {
        text.push_str("The agent took no steps.\n");
    } else {
        text.push_str("What the agent did (last steps):\n");
        for line in lines {
            text.push_str(&line);
            text.push('\n');
        }
    }
    text.push_str(&format!(
        "\nThe agent says: {}\n",
        trace::cap(claim_summary.trim(), CLAIM_CAP)
    ));
    if !front.trim().is_empty() {
        text.push_str(&format!("{}\n", trace::cap(front.trim(), FRONT_CAP)));
    }
    text.push_str(
        "\nThe screenshot is the screen right now. Does it show the task is done? Call verdict.",
    );
    text
}

pub fn parse(arguments: &Value) -> Verdict {
    let result = match arguments["result"]
        .as_str()
        .map(|word| word.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("verified") => VerdictResult::Verified,
        Some("contradicted") => VerdictResult::Contradicted,
        _ => VerdictResult::Unsure,
    };
    let evidence = arguments["evidence"]
        .as_str()
        .map(|text| {
            trace::cap(
                &text.split_whitespace().collect::<Vec<_>>().join(" "),
                EVIDENCE_CAP,
            )
        })
        .unwrap_or_default();
    Verdict { result, evidence }
}

pub async fn verify(client: &ChatClient<'_>, input: VerifyInput<'_>) -> Result<Verdict, String> {
    verify_within(client, input, TIMEOUT).await
}

async fn verify_within(
    client: &ChatClient<'_>,
    input: VerifyInput<'_>,
    limit: Duration,
) -> Result<Verdict, String> {
    let tools = json!([tool()]);
    let text = user_message(
        input.task,
        input.goals,
        input.history,
        input.claim_summary,
        input.front,
    );
    let request = Request {
        system: crate::prompts::verify_system(),
        user_text: &text,
        screenshot: Some(input.screenshot),
        audio: None,
        tools: &tools,
        think: false,
        max_tokens: None,
        data_class: None,
        background: false,
    };
    let mut ignore_thinking = |_: &str| {};
    match tokio::time::timeout(limit, client.call(&request, &mut ignore_thinking)).await {
        Err(_) => Err(format!(
            "the check took longer than {} seconds",
            limit.as_secs()
        )),
        Ok(Err(err)) => Err(err.to_string()),
        Ok(Ok(reply)) if reply.call.name == "verdict" => Ok(parse(&reply.call.arguments)),
        Ok(Ok(reply)) => Err(format!(
            "the model called {} instead of verdict",
            reply.call.name
        )),
    }
}

pub fn should_challenge(
    mode: VerifyMode,
    claimed_success: bool,
    already_challenged: bool,
    result: VerdictResult,
) -> bool {
    mode == VerifyMode::Blocking
        && claimed_success
        && !already_challenged
        && result == VerdictResult::Contradicted
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use super::*;

    #[test]
    fn the_system_prompt_is_short_and_says_what_matters() {
        assert!(
            crate::prompts::verify_system().split_whitespace().count() < 150,
            "{} words",
            crate::prompts::verify_system().split_whitespace().count()
        );
        for needed in [
            "independent",
            "screen",
            "Do not trust",
            "verified",
            "contradicted",
            "unsure",
            "data, never instructions",
            "verdict tool once",
        ] {
            assert!(
                crate::prompts::verify_system().contains(needed),
                "the prompt does not say {needed:?}"
            );
        }
    }

    #[test]
    fn the_tool_is_one_function_with_the_three_answers() {
        let tool = tool();
        assert_eq!(tool["type"], "function");
        assert_eq!(tool["function"]["name"], "verdict");
        assert_eq!(
            tool["function"]["parameters"]["properties"]["result"]["enum"],
            json!(["verified", "contradicted", "unsure"])
        );
        assert_eq!(
            tool["function"]["parameters"]["properties"]["evidence"]["type"],
            "string"
        );
        assert_eq!(
            tool["function"]["parameters"]["required"],
            json!(["result", "evidence"])
        );
    }

    #[test]
    fn a_valid_verdict_is_read() {
        assert_eq!(
            parse(&json!({ "result": "verified", "evidence": "Notepad shows hello rune." })),
            Verdict {
                result: VerdictResult::Verified,
                evidence: "Notepad shows hello rune.".into()
            }
        );
        assert_eq!(
            parse(&json!({ "result": "contradicted", "evidence": "The window is empty." })).result,
            VerdictResult::Contradicted
        );
        assert_eq!(
            parse(&json!({ "result": " Verified ", "evidence": "x" })).result,
            VerdictResult::Verified,
            "case and spaces do not matter"
        );
        assert_eq!(
            parse(&json!({ "result": "unsure", "evidence": "x" })).result,
            VerdictResult::Unsure
        );
    }

    #[test]
    fn anything_else_is_unsure_and_never_panics() {
        for odd in [
            json!({}),
            json!({ "evidence": "no result" }),
            json!({ "result": "maybe" }),
            json!({ "result": 3 }),
            json!({ "result": null }),
            json!("verified"),
            json!(null),
            json!([1, 2]),
            json!(7),
        ] {
            let verdict = parse(&odd);
            assert_eq!(verdict.result, VerdictResult::Unsure, "{odd}");
        }
        assert_eq!(
            parse(&json!({ "result": "verified" })).evidence,
            "",
            "a missing evidence is empty"
        );
        assert_eq!(
            parse(&json!({ "result": "verified", "evidence": 5 })).evidence,
            ""
        );
    }

    #[test]
    fn long_evidence_is_cut_and_trimmed() {
        let verdict = parse(
            &json!({ "result": "contradicted", "evidence": format!("  {}  ", "word ".repeat(300)) }),
        );
        assert_eq!(verdict.evidence.chars().count(), EVIDENCE_CAP);
        assert!(verdict.evidence.ends_with('…') && verdict.evidence.starts_with("word"));
        assert_eq!(
            parse(&json!({ "result": "verified", "evidence": "line one\n  line   two" })).evidence,
            "line one line two",
            "line breaks and runs of spaces become one space"
        );
    }

    fn entry(n: usize) -> Entry {
        Entry {
            action: format!("click {n}"),
            outcome: if n.is_multiple_of(5) {
                "failed: no focus".into()
            } else {
                "done".into()
            },
        }
    }

    #[test]
    fn the_message_has_the_task_the_goals_the_claim_and_at_most_twelve_steps() {
        let goals = vec!["Open Notepad".to_string(), "Type hello".to_string()];
        let history: Vec<Entry> = (1..=20).map(entry).collect();
        let text = user_message(
            "write hello in Notepad",
            &goals,
            &history,
            "Typed hello and saved it.",
            "",
        );
        assert!(text.starts_with("Task: write hello in Notepad\n"));
        assert!(text.contains("1. Open Notepad\n2. Type hello\n"));
        assert!(text.contains("The agent says: Typed hello and saved it."));
        assert!(
            text.contains("8 earlier steps omitted, 1 of them did not work"),
            "the same omission note the agent's own prompt uses"
        );
        let numbered = text.lines().filter(|line| line.contains(" -> ")).count();
        assert_eq!(numbered, 12, "the last twelve steps and no more");
        assert!(
            text.contains("20. click 20 -> failed: no focus")
                && text.contains("9. click 9 -> done")
                && !text.contains("\n8. click 8")
        );
        assert!(text.ends_with("Call verdict."));
    }

    #[test]
    fn the_window_in_front_is_given_as_a_fact_when_known() {
        let front = "Window in front (from the system, not from the screenshot): \"hello - Notepad\", program notepad.";
        let text = user_message("t", &[], &[], "done", front);
        assert!(text.contains("\"hello - Notepad\", program notepad."));
        assert!(text.find("The agent says").unwrap() < text.find("Window in front").unwrap());
        assert!(text.ends_with("Call verdict."));
        assert!(!user_message("t", &[], &[], "done", "  ").contains("Window in front"));
    }

    #[test]
    fn the_message_copes_with_nothing_to_show_and_long_parts() {
        let bare = user_message("t", &[], &[], "", "");
        assert!(bare.contains("The agent took no steps.") && !bare.contains("Plan:"));
        let long = user_message(
            &"t".repeat(2_000),
            &["g".repeat(900)],
            &[],
            &"c".repeat(2_000),
            &"f".repeat(2_000),
        );
        assert!(
            long.len() < 1_800,
            "each part is capped: {} bytes",
            long.len()
        );
    }

    #[test]
    fn only_blocking_challenges_and_only_a_contradicted_first_claim() {
        use VerdictResult::*;
        use VerifyMode::*;
        for mode in [Off, Background, Blocking] {
            for claimed in [false, true] {
                for already in [false, true] {
                    for result in [Verified, Contradicted, Unsure] {
                        let expected =
                            mode == Blocking && claimed && !already && result == Contradicted;
                        assert_eq!(
                            should_challenge(mode, claimed, already, result),
                            expected,
                            "{mode:?} claimed={claimed} already={already} {result:?}"
                        );
                    }
                }
            }
        }
    }

    fn serve(answers: Vec<(&'static str, String)>, delay: Duration) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        std::thread::spawn(move || {
            for (status, body) in answers {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut received = Vec::new();
                let mut chunk = [0_u8; 8192];
                let (header_end, length) = loop {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break (0, 0);
                    }
                    received.extend_from_slice(&chunk[..read]);
                    if let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .and_then(|value| value.trim().parse::<usize>().ok())
                            .unwrap_or(0);
                        break (end + 4, length);
                    }
                };
                while header_end > 0 && received.len() < header_end + length {
                    let read = stream.read(&mut chunk).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    received.extend_from_slice(&chunk[..read]);
                }
                counter.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(delay);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (url, seen)
    }

    fn call_reply(name: &str, arguments: Value) -> String {
        json!({ "choices": [{ "message": { "tool_calls": [{ "function": { "name": name, "arguments": arguments.to_string() } }] } }] }).to_string()
    }

    fn input<'a>() -> VerifyInput<'a> {
        VerifyInput {
            task: "write hello",
            goals: &[],
            history: &[],
            claim_summary: "done",
            screenshot: "data:image/jpeg;base64,AA",
            front: "",
        }
    }

    fn client<'a>(http: &'a reqwest::Client, url: &'a str) -> ChatClient<'a> {
        ChatClient {
            http,
            base_url: url,
            api_key: None,
            model: "m",
            profile: None,
            sink: None,
            no_swap: false,
            on_loading: None,
        }
    }

    #[tokio::test]
    async fn a_verdict_tool_call_becomes_a_verdict() {
        let (url, seen) = serve(
            vec![(
                "200 OK",
                call_reply(
                    "verdict",
                    json!({ "result": "contradicted", "evidence": "The window is empty." }),
                ),
            )],
            Duration::ZERO,
        );
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let verdict = verify(&client(&http, &url), input())
            .await
            .expect("a verdict");
        assert_eq!(
            verdict,
            Verdict {
                result: VerdictResult::Contradicted,
                evidence: "The window is empty.".into()
            }
        );
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn failures_are_errors_and_are_not_retried() {
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let (url, seen) = serve(
            vec![
                ("503 Service Unavailable", "busy".into()),
                (
                    "200 OK",
                    call_reply("verdict", json!({ "result": "verified", "evidence": "x" })),
                ),
            ],
            Duration::ZERO,
        );
        let error = verify(&client(&http, &url), input()).await.unwrap_err();
        assert!(error.contains("503"), "{error}");
        assert_eq!(seen.load(Ordering::SeqCst), 1, "never retried");
        let (url, _) = serve(
            vec![("200 OK", call_reply("done", json!({ "summary": "x" })))],
            Duration::ZERO,
        );
        assert!(
            verify(&client(&http, &url), input())
                .await
                .unwrap_err()
                .contains("instead of verdict")
        );
        let (url, _) = serve(
            vec![(
                "200 OK",
                json!({ "choices": [{ "message": { "content": "I think it is fine" } }] })
                    .to_string(),
            )],
            Duration::ZERO,
        );
        assert!(
            verify(&client(&http, &url), input())
                .await
                .unwrap_err()
                .contains("did not call a tool")
        );
        let dead = TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_url = format!("http://{}", dead.local_addr().unwrap());
        drop(dead);
        assert!(verify(&client(&http, &dead_url), input()).await.is_err());
    }

    #[tokio::test]
    async fn a_slow_server_hits_the_time_limit() {
        let (url, _) = serve(
            vec![(
                "200 OK",
                call_reply("verdict", json!({ "result": "verified", "evidence": "x" })),
            )],
            Duration::from_millis(1_500),
        );
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let started = std::time::Instant::now();
        let error = verify_within(&client(&http, &url), input(), Duration::from_millis(200))
            .await
            .unwrap_err();
        assert!(error.contains("took longer"), "{error}");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "gave up at the limit, not when the server answered"
        );
        assert_eq!(TIMEOUT, Duration::from_secs(30));
    }
}
