# rune-infer TUI: design spec (cleaner, still informative)

For the rune-infer project manager. Written by the Rune Studio orchestrator at the owner's request, from one screenshot of the current TUI (rune-infer v0.6.0, taken 2026-10-20) and Rune Studio's design language (`rune-design-language.md`, read that first; this spec applies it). It describes how the TUI should **look and read**. It does **not** set what rune-infer does, which data it keeps or how the TUI is built: those are yours. Everything about the current TUI comes from the screenshot only; I have not seen the code or other screens, and anything beyond the screenshot is **NOT CERTAIN**. Numbers in the mockups are illustrative, taken from the screenshot where they appear.

## 1. What the current TUI shows (from the screenshot)

- A header box: logo, `Rune Infer v0.6.0`, a green dot, `http://127.0.0.1:8080`, `no API key (localhost only)`, `up 03:21:03`.
- **Models** box: `43 configured, 0 loaded`; a table (Model, Type, Source, Reqs, Err, tok/s) with 8 of the 43 rows visible, a hollow dot for each unloaded model, some names cut off (`google/embeddingge`, `granite-embedding-`, `gliguard-LLMGuardr`, `mmbert32k-intent-c`), mostly zeros and dashes.
- **Performance** box: `VRAM 783 MiB / 15.9 GiB` with a one-cell green block, `Requests 82`, `Errors 0`, `Avg 7147 ms`, a blue "Generation speed" bar chart (last 0 tok/s) and a yellow "Request latency (ms)" bar chart.
- **Recent requests** box: five rows of time, model, status `200` (green), latency, in, out, tok/s.
- **Activity** box: log lines with `INFO` and `WARN`, including an idle unload of `gemma-4-12b-it` after 300 s with the VRAM released.
- Footer: `Tab dashboard / logs`, `?  help`, `q  quit`.
- Colours in use: blue titles and chart, green status and dot, yellow chart and warning, white and grey text, grey rounded frames around every block.

## 2. Where it departs from the Rune design language

| Principle | Current screen | Direction |
| --- | --- | --- |
| Dots are the language | Block cells and bar charts for VRAM, speed and latency. (The hollow dot for an unloaded model already fits.) | Dots for state, quantity and history |
| Monochrome, one accent | Blue, green, yellow and white all carry meaning at once | Grey scale plus one accent; red only for attention and failure |
| Calm and compact | Five framed boxes at the same visual weight; 8 rows of a 43-row table that are almost all zeros | One frame at most; show what is happening now; hide zeros and idle things behind one line |
| Honest | `In 1, Out 1, 0.2 tok/s` on every request looks like data, but the server's token counts are known to be unreliable (see 7) | Do not show a figure that is known to be wrong, or mark it |
| No alarm without cause | `WARN Shutting down llama-server ...` is an expected idle unload | A normal event is not a warning |
| Readable without colour | Status relies on green text; chart colour carries meaning | Always the word as well: `200 OK`, `FAILED` |
| Truncation | Names cut in the middle of a word without a sign | Never cut silently; show an ellipsis and the full name on selection |

## 3. Target: clean yet informative

Rules for what appears:
1. **Now before history.** The first screen answers: is it up, what is loaded, how much VRAM is used, is something running right now, is anything wrong. History comes after.
2. **Zero is quiet.** Models with no traffic and no load are not rows; they are one summary line (`41 more, idle`) with a key to open the full list.
3. **One level of frame at most.** Use blank lines and a dim rule between sections instead of a box around each. If a frame is needed, use rounded corners.
4. **One accent.** Off-white for emphasis, dim grey for secondary, red for failure or attention. The "accent" for live activity is off-white, not a colour.
5. **Numbers a person reads.** Seconds above one second (`5.5 s`), not `5474 ms`; `0.8 of 15.9 GiB`; counts with units. Align decimals.
6. **Truthful.** If a figure is not reliable, show `-` or a marked value; never a precise-looking wrong number.

## 4. Layout (80 columns by 24 rows; grows gracefully)

### 4.1 Dashboard, idle (the screenshot's moment, redrawn)

```
 ·  RUNE INFER 0.6.0      ● READY    127.0.0.1:8080    LOCAL · NO KEY      UP 03:21:03

 MODELS      0 OF 43 LOADED
 ○  gemma-4-12b-it         omni         82 REQ   0 ERR
 ○  liquid-d1-3b           decision      0 REQ
    + 41 MORE · NO TRAFFIC                                            M  LIST ALL

 VRAM        ●···················    0.8 OF 15.9 GIB          NOTHING LOADED

 REQUESTS    82      ERRORS 0      AVG 7.1 S
             ⣀⣀⣄⣄⣤⣤⣴⣶⣶⣷⣷⣿⣶⣴⣤⣄⣀⣀⣄⣤⣶⣷    LAST 60

 15:30:23    gemma-4-12b-it       ● 200 OK     5.5 S
 15:30:18    gemma-4-12b-it       ● 200 OK     7.9 S
 15:30:09    gemma-4-12b-it       ● 200 OK     5.3 S

 ACTIVITY
 ·  15:35:29   gemma-4-12b-it unloaded after 300 S idle · VRAM released
 ·  15:30:23   gemma-4-12b-it request completed

 TAB  LOGS      /  FILTER      ?  HELP      Q  QUIT
```

### 4.2 Dashboard, busy (a model loaded and a request running)

```
 ·  RUNE INFER 0.6.0      ● READY    127.0.0.1:8080    LOCAL · NO KEY      UP 03:26:41

 MODELS      1 OF 43 LOADED
 ●  gemma-4-12b-it         omni         83 REQ   0 ERR     RUNNING  00:04
    + 42 MORE · NO TRAFFIC                                            M  LIST ALL

 VRAM        ●●●●●●●●●●●●●···†····    9.4 OF 15.9 GIB          gemma ●●●●●●●●●●●●●
 QUEUE       ●○○○                                              1 RUNNING · 0 WAITING

 (the requests, graph and activity sections as above)
```
`†` is only an example of drawing a second, smaller allocation; any consistent scheme is fine. Each lit dot is a fixed share of total VRAM (here 5%); the legend names which model owns which dots.

### 4.3 While a model loads or swaps
A small rotating dot arc (the thinking motion from Studio's orb, in braille) next to the model, the words `LOADING gemma-4-12b-it  00:07`, and, if known, `EVICTING liquid-d1-3b`. No red: a swap is normal.

### 4.4 Logs (Tab)
Plain lines, newest at the bottom, dim time, one dot per level: `·` info, `!` for attention (off-white, red only for errors), no coloured level words. `/` filters, Space pauses, repeated lines collapsed with a count (`x14`). Filters by model or by level.

### 4.5 Model list (M)
One line per model: status dot, name in full (widest first, wraps nothing, an ellipsis with the full name shown in a detail line on selection), type, source, **LOCAL or CLOUD**, five capability dots (image in, audio in, speech out, tools, thinking), requests, errors, size in VRAM if known. Models are interchangeable, so this list is how an owner sees at a glance what each one can do (data from INF-7). Sorted by loaded, then by traffic, then name. `/` searches, `T` cycles type filter, Enter shows a detail view: id, engine, capabilities, size, last load time.

### 4.6 Narrow (under 80 columns) and wide (over 120)
Under 80: stack sections, drop the sparkline and the activity block, never wrap a row. Over 120: put MODELS and VRAM side by side; the rest stays a single column. No section ever depends on width to be understood.

### 4.7 No colour, ASCII, piped
No colour: the words and dots carry meaning (`READY`, `FAILED`, `●`/`○`). ASCII fallback: `*` lit, `o` hollow, `.` unlit, `#` for graph levels. Piped or `--json`: no dots, colour or animation; plain data only.

## 5. Dot vocabulary

| Meaning | Glyph | Notes |
| --- | --- | --- |
| Loaded or running | `●` | Off-white |
| Not loaded | `○` | Dim |
| Unlit dot in a gauge | `·` | Dim |
| Failure or unhealthy | `●` in red, plus the word | Red only here |
| Quantity (VRAM, queue, quota) | A row of `●` and `·`, fixed width, each dot a fixed share | Say the share in the legend or help |
| History (latency, speed) | One braille sparkline per metric (`⣀ ⣄ ⣤ ⣴ ⣶ ⣷ ⣿`) with the number beside it | Replace the two bar charts; scale is stated (`LAST 60`) |
| Busy | A braille arc rotating, slow (about 2 steps per second) | Off in no-animation mode |

## 6. Colour

Default foreground for text; dim grey for secondary text, rules, unlit dots and times; off-white for emphasis and lit dots; red for failure and for something that needs the user. That is the whole palette. No blue, green or yellow. Honour `NO_COLOR`; detect colour depth; never set a background.

## 7. What the screenshot reveals about the data (for the PM)

1. **Token figures look wrong.** Every recent request shows `In 1` and `Out 1` and `0.2 tok/s`. Rune Studio's field test found the same: `completion_tokens` was 1 for 62 of 62 steps and `prompt_tokens` 1 (INF-2). The TUI should not show a throughput derived from wrong counts; show `-` until the counts are reliable.
2. **A normal event reads as a warning.** The idle unload is logged at `WARN`. Studio's idle reflection and review tooling would prefer a normal event to be `INFO`, with `WARN` kept for things that need attention.
3. **Idle unload after 300 s.** The log shows `Unloaded idle model ... (no requests for 300s)`. The next request pays a cold load (Studio measured 7 to 16 s for its chat model). Studio would like this timeout to be configurable per model and visible in the TUI (`idle unload in 4:12`), which fits INF-4 (pinning, keep-alive).
4. **VRAM.** `783 MiB / 15.9 GiB` is shown with 0 models loaded: a short note says what the 783 MiB is (driver, other programs, a leftover?), because a user will ask.

## 8. Data the library should provide (so the TUI is a thin view)

Server: version, address, auth mode, uptime, ready state. Per model: id, type, engine, loaded or not, VRAM size if known, last load time, idle-unload deadline, requests, errors. Per request: time, model, status, duration, token counts **marked reliable or not**, queue wait. Aggregates: count, errors, average, and a rolling window for the sparkline. VRAM total and used, and what owns it. Queue: running and waiting. A log stream with level, time, model and a message. These same facts serve rune-infer's API and Rune Studio (INF-4, INF-12).

## 9. Behaviour

Data refreshes about once a second; animation at most 2 to 4 steps a second; a no-animation flag and environment variable; resizing redraws without flicker; every action by keyboard with a visible hint line; `q` and Esc always leave; quitting the TUI never stops the server (say so in help). No mouse requirement.

## 10. Acceptance (visual review by Studio)

Send screenshots of: idle, busy, loading, the model list, the logs, 80 columns, with colour and with `NO_COLOR`, and in the ASCII fallback. Studio checks each against sections 2 to 6 and replies with the differences only. The check is visual and subjective; it decides nothing about features.
