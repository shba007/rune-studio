# Screen Insight

Select API mode, configure an explicit Analysis role model in General settings,
then click Screen Analysis. Choose a screen/window and optionally enter a question;
press Ask to capture one frame. Default question: "Summarize what is visible and identify."

Infer must advertise image input and tool calling for that model. The same
captured frame is sent to the configured model and shown with the answer, capture
time and actual model identity. Findings are observations, not verified facts;
image instructions never authorize actions. This workflow exposes no action or
Kit tools to the model. Kit is not needed for Screen Insight.

Frames stay in memory, are cleared on close, and are not added to screenshot
archives or video. There is no background/continuous capture or change watcher.
A new run cancels/supersedes the previous run. Cancel and Stop discard late model
or capture results. Missing permissions/targets, invalid model capabilities,
Infer errors/busy responses, and timeouts remain visible errors.

The small typed static registry in `src-tauri/src/workflow.rs` supplies identity,
validation, capabilities and execution; its runner owns timeout/cancellation and
presentation lifecycle. Adding a future workflow requires implementing the
interface and static registration, not changing the runner. Only Screen Insight
is registered. Later candidates (not implemented): selected text explanation,
clipboard summarization, and explicit on-demand document inspection. No designer,
DAG engine, scheduler, marketplace or remote multi-user platform is planned here.

Windows is locally validated. macOS/Linux capture permissions and window-capture
backend support require owner platform tests. Minimized/closed windows fail
rather than silently capturing another target. Linux builds need PipeWire/SPA,
Clang, WebKit/GTK, GBM/EGL, XCB, libxdo and ALSA development packages.

## Playground

`/playground` opens a separate window for testing pipelines one at a time. Ask, Point and Pose run the single Screen Insight workflow; every other listed pipeline is a disabled placeholder.

- The Analysis model and image resolution chosen there apply to that run only and never change settings. The model must still advertise image input and tool calling.
- Sources are Screen, Window and File. A File is re-encoded to PNG by the window and sent as an in-memory image. It is never written to disk or archived, and it never draws the on-screen overlay (its frame uses monitor id `u32::MAX`). Importing a file goes beyond the screen/window capture contract and is an owner-approved exception.
- History is the current session only, held in memory.
- The window is not always-on-top and is hidden from capture.