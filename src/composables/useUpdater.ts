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

const STARTUP_DELAY_MS = 5000
const CHECK_TIMEOUT_MS = 15000

const version = ref('')
const phase = ref<UpdatePhase>('idle')
const latest = ref<{ version: string; notes: string } | null>(null)
const progress = ref<number | null>(0)
const error = ref<string | null>(null)
const prompt = ref(false)

let handle: Update | null = null
let started = false

const updating = computed(() => ['downloading', 'installing', 'restarting'].includes(phase.value))

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
    phase.value = silent ? 'idle' : 'error'
    error.value = silent ? null : String(err)
  }
}

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