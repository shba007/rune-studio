# What you can do with Rune Studio

One page per use case. Each page stands alone and has the same sections in the same order: What it is, How to use (always numbered steps), What happens, What you will see, How well it works, Limits, Where it can grow, Features used.

| Page | What you do | Status |
| --- | --- | --- |
| [01 Open an app and type](01-open-app-and-type.md) | "Open Notepad and write hello" | Works |
| [02 System settings and calculator](02-system-settings.md) | "Open Settings", "12 times 13" | Works |
| [03 Browse the web](03-browse-the-web.md) | "Close the cookie banner" | Partly works |
| [04 Risky actions need your Yes](04-risky-actions.md) | "Delete that file" | Partly works |
| [05 Talk to it](05-voice.md) | Say the task out loud | Partly works |
| [06 Let another AI drive](06-outside-agents.md) | Claude Code or Antigravity uses your PC | Works |
| [07 Ask about your screen](07-screen-analysis.md) | "What is on my screen?" | Partly works |
| [08 See what it did](08-review-what-happened.md) | "Why did that fail?" | Works |
| [09 Measure its quality](09-measure-quality.md) | "Did my change make it worse?" | Partly works |
| [10 Record a demo](10-record-a-demo.md) | Make a screen video | Planned (R5) |
| [11 Idle self-review](11-idle-reflection.md) | It reviews its own work when idle | Planned (R5, R6) |

## How to read the evidence

- **Measured** numbers come from the field test of 2026-10-19: Windows 11, one 1920x1080 monitor at 100%, rune-infer 0.6.0 with one model loaded at a time, model `gemma-4-12b-it`, a development build.
- That test ran **before** the fixes in milestone F1. Where F1 changed something, the page says "fixed, not yet seen working on Windows".
- **Not measured** means nobody has tried it yet. **Planned** means it is not built.
- The step-by-step flows are explained from the code, not copied from a recording.

## Things true for every use case

- Rune picks **one action at a time**: look at the screen, choose an action, check it with the safety gate, do it, repeat.
- Works on **Windows 11** only.
- How good it is depends on the model that rune-infer serves.
- Everything Rune sees and hears is **stored on your disk**.
- You can press **Stop** (Ctrl+Alt+Esc by default) at any moment.

## Add a real example run

Run the task, then:

```
rune-studio-cli runs
rune-studio-cli run <id>
```

Paste the step table into the page with the run id and date. Do not edit the numbers.
