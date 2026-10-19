import { getVersion } from '@tauri-apps/api/app'
import { relaunch } from '@tauri-apps/plugin-process'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { computed, ref } from 'vue'

export type UpdatePhase =
  | 'idle'
  | 'checking'
  | 'available'
  | 'uptodate'
  | 'downloading'
  | 'installing'
  | 'restarting'
  | 'error'

/** Wait a little after launch so the check never competes with the first paint. */
const STARTUP_DELAY_MS = 5000
const CHECK_TIMEOUT_MS = 15000

// One shared state, so App.vue, the orb and Settings all see the same update.
const version = ref('')
const phase = ref<UpdatePhase>('idle')
const latest = ref<{ version: string; notes: string } | null>(null)
/** Download progress 0..1, or null while the size is not known. */
const progress = ref<number | null>(0)
const error = ref<string | null>(null)
/** An update was found at start-up and the user has not answered yet. */
const prompt = ref(false)

let handle: Update | null = null
let started = false

const updating = computed(() => ['downloading', 'installing', 'restarting'].includes(phase.value))

/**
 * Looks for a newer version. `silent` (the start-up check) never reports "up to date" or a
 * failure, and is the only kind that raises the prompt.
 */
async function checkForUpdate(silent = false) {
  if (updating.value || phase.value === 'checking') return
  phase.value = 'checking'
  error.value = null
  try {
    const found = await check({ timeout: CHECK_TIMEOUT_MS })
    if (found) {
      handle = found
      latest.value = { version: found.version, notes: found.body ?? '' }
      phase.value = 'available'
      prompt.value = silent
    } else {
      handle = null
      latest.value = null
      phase.value = silent ? 'idle' : 'uptodate'
    }
  } catch (err) {
    // Offline or no release yet is nothing to bother anyone about at launch.
    phase.value = silent ? 'idle' : 'error'
    error.value = silent ? null : String(err)
  }
}

/**
 * Downloads and installs the update found by the last check, then restarts the app. On Windows
 * the plugin quits the app itself as soon as the installer starts, so the restart there is the
 * installer's job (see `plugins.updater.windows` in tauri.conf.json); on macOS and Linux
 * `relaunch()` below does it. `busy` is true while a task runs: updating then would cut it short.
 */
async function installUpdate(busy = false) {
  if (!handle || updating.value || busy) return
  prompt.value = false
  let total = 0
  let received = 0
  progress.value = 0
  phase.value = 'downloading'
  try {
    await handle.downloadAndInstall((event) => {
      if (event.event === 'Started') {
        total = event.data.contentLength ?? 0
        progress.value = total ? 0 : null
      } else if (event.event === 'Progress') {
        received += event.data.chunkLength
        progress.value = total ? Math.min(received / total, 1) : null
      } else {
        progress.value = 1
        phase.value = 'installing'
      }
    })
    phase.value = 'restarting'
    await relaunch()
  } catch (err) {
    phase.value = 'error'
    error.value = String(err)
  }
}

/** Reads the running version and, a moment later, checks for an update once. */
function startup() {
  if (started) return
  started = true
  void getVersion()
    .then((value) => (version.value = value))
    .catch(() => undefined)
  setTimeout(() => void checkForUpdate(true), STARTUP_DELAY_MS)
}

export function useUpdater() {
  return {
    version,
    phase,
    latest,
    progress,
    error,
    prompt,
    updating,
    startup,
    check: checkForUpdate,
    install: installUpdate,
    dismissPrompt: () => (prompt.value = false),
  }
}

export type Updater = ReturnType<typeof useUpdater>

/** Test hook: forget everything, as if the app had just been opened. */
export function resetUpdaterForTests() {
  version.value = ''
  phase.value = 'idle'
  latest.value = null
  progress.value = 0
  error.value = null
  prompt.value = false
  handle = null
  started = false
}
