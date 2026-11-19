# Calendar tool contract (Focus mode, cycle 7)

Studio mirrors the user's calendar into its Focus commitments through one
read-only Kit tool. No calendar plugin exists yet; this is what `rune-calendar`
(in `rune-tools`, a sibling repository) must provide. Studio is already built
against it and shows "No calendar tool is selected" until the tool is installed
and selected in Tools settings.

## Tool

- Name: `get_upcoming_events` (Kit tool names are not prefixed, like
  `get_current_time` in `rune-time`).
- Read-only. It must never create, change or delete an event, and must be
  marked read-only so Kit lets a background task call it.
- Input: `{ "hours": integer }`, how far ahead to look. Studio sends `48`.
- Source: a local `.ics` file or folder chosen in the plugin's configuration.
  Recurring events are **expanded by the plugin** for the window asked for
  (RRULE, EXDATE and moved single occurrences), so Studio never parses
  recurrence rules.

## Output

`structuredContent` (or the same JSON as the first text item):

```json
{
  "events": [
    {
      "id": "abc123@calendar#2026-11-20T09:00:00Z",
      "title": "Design review",
      "startMs": 1795165200000,
      "endMs": 1795168800000,
      "allDay": false
    }
  ]
}
```

- `id`: stable for the same event occurrence between calls. For a recurring
  event, include the occurrence start (as above). Studio updates a commitment in
  place by `id`, so an unstable id makes the wrap-up warning fire again.
- `startMs` / `endMs`: Unix milliseconds, UTC.
- `allDay`: all-day events are ignored by Studio (they block no time).
- Events that ended, or start later than the window, are ignored by Studio.

## What Studio does with it

- Calls the tool about 20 seconds after launch and then every 15 minutes, and
  on the "Sync calendar" button in the Focus card.
- Keeps at most 50 events, titles cut to 120 characters.
- Mirrors them as commitments marked `calendar`: free time, suggestions and the
  wrap-up warning treat them like typed-in ones. They cannot be removed in
  Studio; change them in the calendar.
- A failed or malformed read leaves the commitments as they were. An empty list
  is a valid answer and clears the synced commitments (typed-in ones stay).