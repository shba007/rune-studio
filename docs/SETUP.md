# Connect an AI agent to Rune Studio

This page is for you, the person using the PC. You need only the Rune Studio files: no Node, Python, Rust or other developer tool.

Rune Studio lets an AI agent (such as Claude Code) see your screen and use your mouse and keyboard, while you watch and keep a Stop button. This needs the Rune Studio build with agent support. It comes as two programs in one folder: `rune-studio-app.exe` (the app) and `rune-studio.exe` (the command).

## Set up (once)

Open a terminal in the Rune Studio folder, or put that folder on your PATH, then run:

```bash
rune-studio connect claude
```

That tells Claude Code about Rune Studio for **every project**, and allows outside agents. Then restart Claude Code. Ask it to do something on your PC and Rune Studio opens by itself.

Using a different agent? `rune-studio connect --print` shows the settings to paste into it.

## Everyday commands

| Command               | What it does                                                             |
| --------------------- | ------------------------------------------------------------------------ |
| `rune-studio`         | Opens the app.                                                           |
| `rune-studio status`  | Shows whether the app is running and whether outside agents are allowed. |
| `rune-studio enable`  | Allows outside agents.                                                   |
| `rune-studio disable` | Stops allowing them.                                                     |
| `rune-studio guide`   | Prints the guide the agent reads.                                        |
| `rune-studio help`    | Lists these commands.                                                    |

You can also switch **Allow outside agent** on or off in the app's Settings.

## You stay in control

- While an agent works you see a frame around the screen, a caption for each action, and a ripple where it clicks.
- Press **Stop** (the red button in the app, the tray menu, or **Ctrl+Alt+Esc**) and the agent loses control at once. It cannot start again until you turn the switch off and on (or run `rune-studio enable` again).
- Only one agent can work at a time, and an agent that goes quiet for 5 minutes loses control.
- Agents only reach this PC through Rune Studio, and only while the switch is on.