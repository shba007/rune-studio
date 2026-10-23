use std::io::{self, BufRead, Write};
fn main() {
    if std::env::args().any(|a| a == "--version") {
        println!("rune-kit 0.5.0");
        return;
    }
    let mode = std::env::current_exe()
        .unwrap()
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        let Some(at) = line.find("\"id\":") else {
            continue;
        };
        let id: String = line[at + 5..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        assert!(
            line.contains("io.modelcontextprotocol/protocolVersion")
                && line.contains("2026-07-28")
                && line.contains("io.modelcontextprotocol/clientCapabilities")
        );
        assert!(!line.contains("\"initialize\""));
        if (mode.contains("discovery-error") && line.contains("tools/list"))
            || (mode.contains("rpc-cancel") && line.contains("tools/call"))
            || (mode.contains("rpc-refuse") && line.contains("tools/call"))
        {
            let code = if mode.contains("rpc-cancel") {
                -32800
            } else if mode.contains("rpc-refuse") {
                -32600
            } else {
                -32000
            };
            println!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":\"fixture error\"}}}}"
            );
            continue;
        }
        let result = if line.contains("\"server/discover\"") {
            r#"{"supportedVersions":["2026-07-28"],"capabilities":{"tools":{}},"ttlMs":0,"cacheScope":"private"}"#.to_string()
        } else if line.contains("\"tools/list\"") {
            r#"{"tools":[{"name":"time__get_current_time","description":"time","annotations":{"readOnlyHint":true,"destructiveHint":false},"inputSchema":{"type":"object","properties":{"timezone":{"type":"string"}},"additionalProperties":false,"x-rune-rules":{"read_only":true,"risk":"low","effects":["read"]}}}]}"#.to_string()
        } else if line.contains("\"tools/call\"") {
            if mode.contains("disconnect") {
                return;
            }
            if mode.contains("ignore") {
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            if mode.contains("malformed") {
                println!("invalid json");
                continue;
            }
            if mode.contains("error") {
                r#"{"isError":true,"content":[{"type":"text","text":"plugin failed"}]}"#.to_string()
            } else if mode.contains("oversized") {
                format!(
                    "{{\"content\":[{{\"type\":\"text\",\"text\":\"{}\"}}]}}",
                    "x".repeat(70000)
                )
            } else {
                r#"{"content":[{"type":"text","text":"2026-10-23T00:00:00Z"}]}"#.to_string()
            }
        } else {
            continue;
        };
        let result = format!("{{\"resultType\":\"complete\",{}", &result[1..]);
        let response = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}\n");
        let mid = response.len() / 2;
        io::stdout().write_all(&response.as_bytes()[..mid]).unwrap();
        io::stdout().flush().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        io::stdout().write_all(&response.as_bytes()[mid..]).unwrap();
        io::stdout().flush().unwrap();
    }
}
