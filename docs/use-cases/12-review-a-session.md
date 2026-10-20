# Review a session

**Status:** Built and tested in code. Never seen on a real Windows screen, and the video encoder has never run there.

## What it is

When a task ends, Rune offers to replay it: a video of the session, a timeline of every step, and the details behind each step (what the model was told, what it answered, how safe the action was judged, and every try of every model call, retries included).

## How to use

1. Run any task. When it ends, a small card appears above the orb with the task, how it ended and the time and step count.
2. Click **Review**. Click **Dismiss**, or wait 15 seconds: the dots on the card run out and it goes away.
3. The Flight Player opens in its own window. Press **Space** to play or pause, **Left** and **Right** to go to the previous or next step, **J** and **K** to slow down or speed up.
4. Click or drag on the timeline to jump. Roll the mouse wheel over it to zoom in.
5. Press **D** (or click **Details**) to open the drawer. Press **T** to move through its tabs. Click **Raw** to see the stored JSON, and **Copy** to copy it.
6. Press **A** to hide or show the small overlay of what Rune did (the click ripple, the typed text, the keys).
7. Typed text is hidden. Click **Typed text: hidden** (or **Show** next to the arguments) only when nobody else can see the window.
8. Press **Esc** to close the drawer, then again to close the window.
9. To look at an older task, choose **Open Flight Player** from the tray icon's menu and pick it from the list.

## What happens

1. When the task ends, a watcher notices the finished run and shows the card. It is hidden while another task runs, and a confirmation or the idle-review question comes first.
2. After the task, once nothing else is running, Rune builds `session.mp4` in the session folder from the screenshots the model looked at, each held for the time it was current. This is a **storyboard**, not smooth motion.
3. The player reads the record (it can only read, never change it) and plays the video. The overlay is drawn from the recorded steps, not burned into the video.
4. If there is no video (an older task, or no encoder on this PC), the player shows each step's screenshot instead and plays them on a clock.

## What you will see

A window with the task and its outcome on top, the picture in the middle, play and speed controls, a track of dots where each step is as wide as it really lasted, one sentence about the current step, and, on demand, a side drawer with the tabs Step, Reasoning, Prompt, Safety, API, Timing, Tools and Files. A tab appears only when the record has something for it. The API tab lists every model attempt with its status, time, wait and outcome; open one to read the request and the reply. A deleted screenshot or a missing video gives a one-line message, not an error.

## How well it works

Not measured on Windows. Measured only in a headless Chromium with a generated WebM file: the Video.js player loaded, played and seeked in 25 to 60 ms per jump. That says nothing about the Windows webview, H.264, or the size of a real video.

## Limits

- The video is a storyboard of about one frame per step. Step times have one-second resolution, so the picture and the timeline agree to about a second.
- The video encoder is Windows Media Foundation. It has not been run, and may be missing on "N" editions of Windows.
- Model requests made by the optional decision model, by voice and by steering are not in the API tab. Kit tool calls and skills have no data yet.
- Images are not kept in the stored request (only their size); the step's screenshot is shown instead.
- The record, including the video and the model bodies, can show anything that was on your screen. The window is hidden from screen capture unless demo mode is on.
- `recordSessionVideo` and `offerReview` are in `settings.json` only (no Settings page yet).
- Full-motion recording (`recordSessionVideo: "motion"`, 2 frames per second) is off by default and its cost is not measured.

## Where it can grow

Export a session, notes on a step, a comparison of two runs, post-mortems written from the record (R6c).

## Features used

The SQLite trace (`runs`, `steps`, `events`, `calls`), session folders, the video encoder, the asset protocol, Video.js v10, the capture exclusion of Rune's windows.