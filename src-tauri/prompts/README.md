# Prompts

Every piece of text sent to a model lives here, one Markdown file per prompt, and is compiled in by `src/prompts.rs` (`include_str!`). Edit the file, rebuild, done: no Rust string escaping.

| Folder      | Used by                                  |
| ----------- | ---------------------------------------- |
| `agent/`    | the agent loop: system, planner, nudges  |
| `skills/`   | reusable snippets composed into prompts  |
| `quick/`    | the decision model's yes/no questions    |
| `verify/`   | the independent check of a finished task |
| `voice/`    | transcription and intent                 |
| `analysis/` | screen analysis                          |
| `schedule/` | reading reminder requests from the composer |

`{name}` marks a slot filled by `prompts::fill`. Files are trimmed, so a trailing newline is fine. The outside-agent guide is `docs/AGENT-GUIDE.md`.