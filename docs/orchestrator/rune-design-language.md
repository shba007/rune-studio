# Rune design language, and how it applies to a terminal interface

For the project managers of rune-infer and rune-kit (and anyone building a Rune interface). Owned by Rune Studio. A worked example for the rune-infer TUI is in `rune-infer-tui-spec.md`. It states **what the look is**, **which parts are fixed**, and **a suggested translation to a terminal**. The owner's decision: the TUI of rune-infer, and any future TUI of rune-kit, follow the design principle of Rune Studio, which is a Nothing-phone style: a dot interface. This document does not tell you what your TUI does or which screens it has; it only describes how it should look and feel. Terminal details are suggestions for you to adapt, marked as such. Anything not verified is marked **NOT CERTAIN**.

## 1. The principle in six lines

1. **Dots are the language.** State, progress and activity are drawn with dots on a grid, not with bars, spinners or icons.
2. **Monochrome first.** Near-black, off-white and greys. One accent at a time, only when it carries meaning.
3. **Red means attention, failure or stop. Nothing else is red.**
4. **Compact and calm.** Small captions in capitals with wide letter spacing, short labels, no decoration, no boxes inside boxes, nothing animates without a reason.
5. **Honest.** Show what is happening and what it costs (time, memory), not a reassuring spinner.
6. **Respect the user.** Reduced motion and plain output are always available; every state is also readable without colour.

## 2. What Rune Studio uses (the source of truth)

| Element | Value in Studio |
| --- | --- |
| Surface (dark) | `#141414`; the control bar is black at 85% opacity |
| Light | `#f1f1ef` |
| Attention / failure / stop | `#d71921` |
| Dots (on light) | `rgb(17,17,17)` with brightness 0 to 1; unlit dots at 13% |
| Typeface | Oxanium (a squared, technical face); caption style is about 10 px, semi-bold, capitals, wide spacing |
| Orb | A disc of an 11 by 11 dot grid. States: idle (a faint ring and a centre dot), listening (grows with the voice level), thinking (an arc of dots rotating), acting (a bright dot moving with a ripple), attention (a pulsing exclamation mark in red), done (a check drawn dot by dot), failed (a cross in red) |
| Progress | A ring of dotted arcs, one per goal |
| Motion | Slow and small; none under "reduce motion" |
| Corners | Large radii (cards about 28 px) |

If these values change, Studio will say so here. Do not copy hex values into your code in many places: keep them in one place so they can change.

## 3. Suggested translation to a terminal (for you to adapt)

**Colour.** Use at most: default foreground, a dim grey, off-white for emphasis, and red for attention/failure. Detect colour depth (true colour, 256, 16, none) and honour `NO_COLOR`; with no colour, meaning must survive (see glyphs). Do not set a background colour: keep the user's terminal background.

**Glyphs (suggested set).** Unlit dot `·`, lit dot `●`, soft dot `•`, hollow dot `○`, and braille cells (`⠁` to `⣿`) for fine dot drawings and spinners. Provide a plain-ASCII fallback (`.` `o` `*`) used when the terminal font or locale cannot show them (**NOT CERTAIN** how often this happens on legacy Windows consoles; Windows Terminal handles them, the old console host may not).

**Dots as the only chart.** Progress `●●●●○○○○`, a level or a quota as a row of dots, a table-free list where each row starts with a status dot. A busy state is a small dot grid that moves, the same idea as Studio's orb at a smaller size (for example a 5 by 5 or braille orb: a ring, a rotating arc for working, a pulsing mark in red for attention, a check for done, a cross in red for failed). Keep the mapping of states to shapes the same as Studio's, so a user who knows one knows the other.

**Type.** The terminal's own monospace font. Captions in capitals with spaces between words are fine; do not simulate wide letter spacing with spaces between every letter.

**Layout.** Compact. Works at 80 columns and 24 rows; uses more space gracefully when available. One screen, one purpose. Short labels. No box-drawing frames around every group: use blank space and a single dim rule. Rounded corners where a frame is needed (`╭ ╮ ╰ ╯`).

**Motion.** Slow and sparse. Provide a no-animation mode, enabled by a flag, an environment variable, or when the output is not a terminal; respect the user's reduced-motion preference where the platform exposes it.

**Keyboard.** Everything reachable by keyboard, visible key hints in a single dim line at the bottom, `q` or Esc always leaves, no mouse requirement.

**Non-interactive output.** When output is piped or `--json` is given, print no dots, colour or animation: plain data only.

**Accessibility.** Never rely on colour or on the dot pattern alone; always also print the word (`DONE`, `FAILED`, `WAITING`). Screen-reader users should be able to use the plain mode.

## 4. Examples of what dots could show (examples, not requirements)

- **rune-infer:** VRAM as a row of dots (used, free), each loaded model as a labelled group of lit dots, a request queue as dots that move to the front, a swap as a visible "loading" arc, health as one dot per backend. These also serve the visibility asked for in INF-4.
- **rune-kit:** installed plugins as a list of status dots (installed, update available, failed), an install or update as a dotted progress row, running tool calls as moving dots, a failed plugin as one red dot with its reason.

## 5. Libraries and the TUI

Both rune-infer and rune-kit have a library; rune-infer has a TUI today and rune-kit has none yet. A request that applies to both (and to a future TUI of kit): the TUI should be a thin consumer of the library (so the look can change without touching logic), and the library should expose what the TUI shows (state and progress as data), so that other clients such as Rune Studio can show the same facts in their own style.

## 6. Checking

Studio offers a short review: send screenshots of your TUI in the colour and no-colour modes and Studio will say where it departs from this document. The check is visual and subjective; Studio decides nothing about your features.
