//! M1.5: can a local vision model click what it is asked to click?
//!
//! For each synthetic scene (see scripts/m15) and each sampled target, ask rune-infer to click it
//! by name, under one or more prompt conventions, and record whether the point it returns lands
//! inside the element's real bounds, and how long that took. Results go to a JSONL file that
//! scripts/m15/report.mjs summarises.
//!
//! Screenshots go through `observe::screenshot::prepare`, the same code path the agent will use.
//! Run through `pwsh scripts/m15/run-probe.ps1`; it sets the environment variables read below.

use std::{
    collections::HashMap,
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::observe::screenshot::{MODEL_JPEG_QUALITY, MODEL_MAX_EDGE, prepare};

/// The scenes are rendered at this size; ground truth is in these pixels.
const STAGE_W: f64 = 1920.0;
const STAGE_H: f64 = 1080.0;
/// A click within this many pixels of the element still counts as "near".
const NEAR_MARGIN: f64 = 12.0;

#[derive(Deserialize)]
struct SceneFile {
    scene: String,
    targets: Vec<Target>,
}

#[derive(Deserialize, Clone)]
struct Target {
    name: String,
    role: String,
    l: f64,
    t: f64,
    w: f64,
    h: f64,
    size: String,
}

/// How the model is told to express a point.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Convention {
    /// 0..1000 across the screenshot, through a `click` tool.
    Norm,
    /// As `Norm`, but the model is told not to explain itself (to cut reasoning tokens and latency).
    Brief,
    /// Pixels of the (downscaled) screenshot, through a `click` tool.
    Pixels,
    /// rune-infer's dedicated `/v1/images/grounding` endpoint (0..1000).
    Ground,
}

#[derive(Clone)]
struct Mode {
    model: String,
    convention: Convention,
    /// Longest edge of the screenshot sent, in pixels.
    edge: u32,
    /// Ask the server not to think (`reasoning_effort: "none"`).
    no_think: bool,
}

impl Mode {
    fn label(&self) -> String {
        let edge = if self.edge == MODEL_MAX_EDGE { String::new() } else { format!("@{}", self.edge) };
        let think = if self.no_think { "+nothink" } else { "" };
        format!("{}:{:?}{edge}{think}", self.model, self.convention).to_lowercase()
    }
}

fn parse_modes(spec: &str) -> Vec<Mode> {
    spec.split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (model, rest) = part.trim().rsplit_once(':').unwrap_or((part.trim(), "norm"));
            let (rest, no_think) = rest.strip_suffix("+nothink").map_or((rest, false), |r| (r, true));
            let (convention, edge) = rest.split_once('@').map_or((rest, MODEL_MAX_EDGE), |(c, e)| (c, e.parse().expect("bad @edge in RUNE_PROBE_MODES")));
            let convention = match convention {
                "norm" => Convention::Norm,
                "brief" => Convention::Brief,
                "px" | "pixels" => Convention::Pixels,
                "ground" => Convention::Ground,
                other => panic!("unknown convention '{other}' in RUNE_PROBE_MODES"),
            };
            Mode { model: model.to_string(), convention, edge, no_think }
        })
        .collect()
}

fn system_prompt(convention: Convention, width: u32, height: u32) -> String {
    let how = match convention {
        Convention::Pixels => format!(
            "Coordinates are pixels of the screenshot, which is {width} wide and {height} tall, with (0, 0) at the top-left."
        ),
        _ => "Coordinates are normalized: x and y each run from 0 to 1000 across the full width and height of the screenshot, \
              with (0, 0) at the top-left and (1000, 1000) at the bottom-right."
            .to_string(),
    };
    let brevity = if convention == Convention::Brief {
        " Do not explain or think aloud. Call the tool immediately with your best estimate."
    } else {
        ""
    };
    format!(
        "You operate a Windows PC by looking at screenshots. To act, call the click tool with the exact centre of the element to click. {how}{brevity}"
    )
}

struct Reply {
    point: Option<(f64, f64)>,
    completion_tokens: Option<u64>,
    reasoning_chars: usize,
    finish: String,
    note: String,
}

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

/// Picks `k` targets that mix sizes: small, medium and large in turn, so small ones are not crowded out.
fn pick(targets: &[Target], k: usize) -> Vec<Target> {
    let mut groups: Vec<Vec<&Target>> = ["small", "medium", "large"]
        .iter()
        .map(|size| targets.iter().filter(|t| t.size == *size).collect())
        .collect();
    let mut picked = Vec::new();
    while picked.len() < k && groups.iter().any(|g| !g.is_empty()) {
        for group in groups.iter_mut() {
            if picked.len() < k && !group.is_empty() {
                picked.push(group.remove(0).clone());
            }
        }
    }
    picked
}

async fn ask(
    client: &reqwest::Client,
    base: &str,
    key: &str,
    mode: &Mode,
    data_uri: &str,
    (width, height): (u32, u32),
    target: &Target,
) -> Result<Reply, String> {
    let description = format!("the \"{}\" {}", target.name, target.role);

    if mode.convention == Convention::Ground {
        let body = json!({ "model": mode.model, "image": data_uri, "query": description });
        let value = post(client, &format!("{base}/v1/images/grounding"), key, &body).await?;
        let point = value["point"].as_array().and_then(|p| Some((number(p.first()?)?, number(p.get(1)?)?)));
        return Ok(Reply {
            point,
            completion_tokens: None,
            reasoning_chars: 0,
            finish: "grounding".into(),
            note: if point.is_none() { value.to_string().chars().take(300).collect() } else { String::new() },
        });
    }

    let axis_hint = if mode.convention == Convention::Pixels { "pixels" } else { "0 to 1000" };
    let mut body = json!({
        "model": mode.model,
        "temperature": 0,
        "max_tokens": 1024,
        "messages": [
            { "role": "system", "content": system_prompt(mode.convention, width, height) },
            // Image first, question last: the screenshot is the shared prefix, so the server can
            // reuse it between questions about the same scene.
            { "role": "user", "content": [
                { "type": "image_url", "image_url": { "url": data_uri } },
                { "type": "text", "text": format!("Click {description}.") },
            ]},
        ],
        "tools": [{ "type": "function", "function": {
            "name": "click",
            "description": "Click a point on the screen.",
            "parameters": { "type": "object", "required": ["x", "y"], "properties": {
                "x": { "type": "integer", "description": format!("Horizontal position, {axis_hint}, left to right") },
                "y": { "type": "integer", "description": format!("Vertical position, {axis_hint}, top to bottom") },
            }},
        }}],
        "tool_choice": "required",
    });
    if mode.no_think {
        body["reasoning_effort"] = json!("none");
    }
    let value = post(client, &format!("{base}/v1/chat/completions"), key, &body).await?;

    let message = &value["choices"][0]["message"];
    let arguments = message["tool_calls"][0]["function"]["arguments"].as_str().unwrap_or("");
    let parsed: Value = serde_json::from_str(arguments).unwrap_or(Value::Null);
    let point = number(&parsed["x"]).zip(number(&parsed["y"]));
    Ok(Reply {
        point,
        completion_tokens: value["usage"]["completion_tokens"].as_u64(),
        reasoning_chars: message["reasoning_content"].as_str().map_or(0, |text| text.chars().count()),
        finish: value["choices"][0]["finish_reason"].as_str().unwrap_or("").to_string(),
        note: if point.is_none() {
            format!("no usable tool call; content: {}", message["content"].as_str().unwrap_or("").chars().take(200).collect::<String>())
        } else {
            String::new()
        },
    })
}

async fn post(client: &reqwest::Client, url: &str, key: &str, body: &Value) -> Result<Value, String> {
    let response = client.post(url).bearer_auth(key).json(body).send().await.map_err(|e| format!("request failed: {e}"))?;
    let status = response.status();
    let text = response.text().await.map_err(|e| format!("could not read the reply: {e}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", text.chars().take(300).collect::<String>()));
    }
    serde_json::from_str(&text).map_err(|e| format!("reply was not JSON ({e}): {}", text.chars().take(200).collect::<String>()))
}

fn load_scenes(dir: &Path, only: &Option<Vec<String>>) -> Vec<(SceneFile, PathBuf)> {
    let mut scenes: Vec<(SceneFile, PathBuf)> = fs::read_dir(dir)
        .expect("scene directory not found; run scripts/m15/make-scenes.mjs")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json") && path.file_name().is_some_and(|n| n != "index.json"))
        .filter_map(|path| {
            let scene: SceneFile = serde_json::from_str(&fs::read_to_string(&path).ok()?).ok()?;
            let png = path.with_extension("png");
            png.exists().then_some((scene, png))
        })
        .filter(|(scene, _)| only.as_ref().is_none_or(|names| names.iter().any(|n| *n == scene.scene)))
        .collect();
    scenes.sort_by(|a, b| a.0.scene.cmp(&b.0.scene));
    scenes
}

#[test]
#[ignore = "talks to a local rune-infer server; run through scripts/m15/run-probe.ps1"]
fn m15_probe() {
    let dir = PathBuf::from(env::var("RUNE_PROBE_DIR").expect("RUNE_PROBE_DIR is not set"));
    let out_path = PathBuf::from(env::var("RUNE_PROBE_OUT").expect("RUNE_PROBE_OUT is not set"));
    let modes = parse_modes(&env::var("RUNE_PROBE_MODES").unwrap_or_else(|_| "qwen3.8-27b:norm".into()));
    let per_scene: usize = env::var("RUNE_PROBE_PER_SCENE").ok().and_then(|v| v.parse().ok()).unwrap_or(4);
    let only = env::var("RUNE_PROBE_SCENES").ok().map(|v| v.split(',').map(str::to_string).collect::<Vec<_>>());
    let base = env::var("RUNE_PROBE_BASE").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    let key = env::var("RUNE_API_KEY").unwrap_or_else(|_| "sk-rune-local-test-key".into());

    let scenes = load_scenes(&dir, &only);
    assert!(!scenes.is_empty(), "no scenes found in {}", dir.display());
    let mut out = fs::OpenOptions::new().create(true).append(true).open(&out_path).expect("cannot open the results file");

    let client = reqwest::Client::builder().timeout(Duration::from_secs(900)).build().unwrap();
    tauri::async_runtime::block_on(async {
        let mut warmed: Vec<String> = Vec::new();
        for (scene, png) in &scenes {
            let rgba = image::open(png).expect("cannot read the scene PNG").to_rgba8();
            let mut prepared_by_edge: HashMap<u32, (String, (u32, u32))> = HashMap::new();
            for mode in &modes {
                prepared_by_edge.entry(mode.edge).or_insert_with(|| {
                    let prepared = prepare(rgba.clone(), mode.edge, MODEL_JPEG_QUALITY).expect("cannot prepare the scene");
                    (format!("data:image/jpeg;base64,{}", BASE64.encode(&prepared.jpeg)), (prepared.width, prepared.height))
                });
            }
            let targets = pick(&scene.targets, per_scene);

            for mode in &modes {
                let (data_uri, dims) = &prepared_by_edge[&mode.edge];
                let (data_uri, dims) = (data_uri.as_str(), *dims);
                // The first request to a model includes loading it; measure and report that apart.
                if !warmed.contains(&mode.model) {
                    let started = Instant::now();
                    let warm = ask(&client, &base, &key, mode, data_uri, dims, &targets[0]).await;
                    println!("warm-up {}: {:?} ms ({})", mode.model, started.elapsed().as_millis(), if warm.is_ok() { "ok" } else { "failed" });
                    warmed.push(mode.model.clone());
                }
                for target in &targets {
                    let started = Instant::now();
                    let reply = ask(&client, &base, &key, mode, data_uri, dims, target).await;
                    let latency_ms = started.elapsed().as_millis() as u64;

                    let mut record = json!({
                        "scene": scene.scene, "mode": mode.label(), "model": mode.model,
                        "target": target.name, "role": target.role, "size": target.size,
                        "box": [target.l, target.t, target.w, target.h], "latency_ms": latency_ms,
                    });
                    match reply {
                        Ok(reply) => {
                            let hit = reply.point.map(|(x, y)| {
                                let (sx, sy) = match mode.convention {
                                    Convention::Pixels => (x / f64::from(dims.0) * STAGE_W, y / f64::from(dims.1) * STAGE_H),
                                    _ => (x / 1000.0 * STAGE_W, y / 1000.0 * STAGE_H),
                                };
                                let inside = |m: f64| sx >= target.l - m && sx <= target.l + target.w + m && sy >= target.t - m && sy <= target.t + target.h + m;
                                let (cx, cy) = (target.l + target.w / 2.0, target.t + target.h / 2.0);
                                (sx, sy, inside(0.0), inside(NEAR_MARGIN), ((sx - cx).powi(2) + (sy - cy).powi(2)).sqrt())
                            });
                            record["ok"] = json!(reply.point.is_some());
                            record["raw"] = json!(reply.point);
                            record["completion_tokens"] = json!(reply.completion_tokens);
                            record["reasoning_chars"] = json!(reply.reasoning_chars);
                            record["finish"] = json!(reply.finish);
                            record["note"] = json!(reply.note);
                            if let Some((sx, sy, inside, near, dist)) = hit {
                                record["screen"] = json!([sx.round(), sy.round()]);
                                record["hit"] = json!(inside);
                                record["near"] = json!(near);
                                record["dist_px"] = json!(dist.round());
                            }
                        }
                        Err(error) => {
                            record["ok"] = json!(false);
                            record["note"] = json!(error);
                        }
                    }
                    println!(
                        "{:<16} {:<28} {:<26} {:<7} {:>6} ms  {}",
                        scene.scene, mode.label(), target.name, target.size, latency_ms,
                        match (record["hit"].as_bool(), record["near"].as_bool(), record["ok"].as_bool()) {
                            (Some(true), _, _) => "HIT",
                            (Some(false), Some(true), _) => "near",
                            (Some(false), _, _) => "miss",
                            _ => "FAILED",
                        }
                    );
                    writeln!(out, "{record}").expect("cannot write the results file");
                }
            }
        }
    });
}
