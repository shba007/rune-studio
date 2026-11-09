<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { BackendProbe, Settings } from '@/types/protocol'
import RuneButton from './RuneButton.vue'
import OnboardingLocalModel from './OnboardingLocalModel.vue'
import OnboardingCloudModel from './OnboardingCloudModel.vue'
import TitleBar from './TitleBar.vue'

type Choice = 'local' | 'cloud'

interface KitConfig {
  enabled: boolean
  endpoint: string
  selected: string[]
}

interface TestPrompt {
  category: string
  prompt: string
  tag: string
}

const logo = '/logo.svg'
const storedStep = Number(localStorage.getItem('rune-onboarding-step') ?? 0)
const step = ref(
  Number.isInteger(storedStep) && storedStep >= 0 && storedStep <= 3 ? Math.min(storedStep, 2) : 0,
)
const savedChoice = localStorage.getItem('rune-onboarding-choice')
const choice = ref<Choice>(savedChoice === 'cloud' ? 'cloud' : 'local')
const busy = ref(false)
const message = ref('')
const capabilityWarning = ref('')
const stopShortcut = ref('Escape')
const localAvailable = ref<boolean | null>(null)
const platform = ref('')
const nextLabel = computed(() => (step.value === 3 ? 'Open Rune' : 'Continue'))

interface Starter {
  name: string
  namespace: string
  title: string
  description: string
  button: string
}

const starters: Starter[] = [
  {
    name: 'rune-fetch',
    namespace: 'fetch',
    title: 'Starter MCP Server: Web Research',
    description: 'Installs rune-fetch to read web pages.',
    button: 'Install Fetch MCP',
  },
  {
    name: 'rune-time',
    namespace: 'time',
    title: 'Starter MCP Server: Clock',
    description: 'Installs rune-time for the current time in any time zone.',
    button: 'Install Time MCP',
  },
]
const mcpBusy = ref('')
const mcpInstalled = ref<Record<string, boolean>>({})
const mcpStatus = ref('')
const mcpFailed = ref(false)
const copiedPrompt = ref('')
let copyTimer: ReturnType<typeof setTimeout> | null = null

const testPrompts: TestPrompt[] = [
  {
    category: 'Computer Use',
    prompt: 'Open a text editor and type hello.',
    tag: 'Desktop',
  },
  {
    category: 'MCP Use',
    prompt: 'Use the fetch MCP to read https://example.com and tell me what the page says.',
    tag: 'Fetch MCP',
  },
  {
    category: 'MCP Use',
    prompt: 'Use the time MCP to tell me the current time in Tokyo.',
    tag: 'Time MCP',
  },
  {
    category: 'Filesystem',
    prompt: "Create a meeting notes file on my desktop with today's date.",
    tag: 'Desktop',
  },
  {
    category: 'Browser Use',
    prompt: 'Open the browser and search for the latest weather.',
    tag: 'Browser',
  },
]

async function startKit(kitConfig: KitConfig) {
  if (await invoke<boolean>('embedded_kit_available')) {
    kitConfig.endpoint = await invoke<string>('start_embedded_kit')
  }
}

async function selectStarterTools(kitConfig: KitConfig, starter: Starter): Promise<number> {
  const discovery = await invoke<{ tools: Array<{ name: string }> }>('kit_discover', {
    config: kitConfig,
  })
  const names = (discovery?.tools ?? [])
    .map((tool) => tool.name)
    .filter((name) => name.split('__')[0]!.includes(starter.namespace))
  kitConfig.enabled = true
  for (const name of names) {
    if (!kitConfig.selected.includes(name)) kitConfig.selected.push(name)
  }
  await invoke('kit_save', { config: kitConfig })
  return names.length
}

async function checkStarterMcp() {
  try {
    const kitConfig = await invoke<KitConfig>('kit_settings')
    await startKit(kitConfig)
    const result = await invoke<{
      mcps?: Array<{ name: string }>
      plugins?: Array<{ name: string }>
    }>('kit_packages', {
      config: kitConfig,
    })
    const list = result.mcps ?? result.plugins ?? []
    for (const starter of starters) {
      if (list.some((pkg) => pkg.name === starter.name || pkg.name.includes(starter.namespace))) {
        mcpInstalled.value = { ...mcpInstalled.value, [starter.name]: true }
        await selectStarterTools(kitConfig, starter)
      }
    }
  } catch {
    // Kit server may not be reachable or configured yet
  }
}

async function installStarterMcp(starter: Starter) {
  mcpBusy.value = starter.name
  mcpStatus.value = ''
  mcpFailed.value = false
  try {
    const kitConfig = await invoke<KitConfig>('kit_settings')
    await startKit(kitConfig)
    await invoke('kit_install_package', {
      config: kitConfig,
      package: {
        kind: 'mcp',
        target: starter.name,
        version: null,
        prefer_native: false,
      },
    })
    mcpInstalled.value = { ...mcpInstalled.value, [starter.name]: true }
    const tools = await selectStarterTools(kitConfig, starter)
    mcpStatus.value = tools
      ? `${starter.name} installed and enabled.`
      : `Installed, but Kit lists no ${starter.namespace} tools yet. Check Settings → Tools.`
  } catch (error) {
    mcpFailed.value = true
    mcpStatus.value = `${String(error).replace(/\.+$/, '')}. You can also configure MCP servers anytime in Settings.`
  } finally {
    mcpBusy.value = ''
  }
}

async function copyPrompt(text: string) {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text)
    }
    copiedPrompt.value = text
    if (copyTimer) clearTimeout(copyTimer)
    copyTimer = setTimeout(() => {
      copiedPrompt.value = ''
    }, 2000)
  } catch {
    // Clipboard permission denied or unsupported
  }
}

onMounted(async () => {
  if (step.value === 3) {
    void checkStarterMcp()
  }
  void Promise.resolve(invoke<string>('onboarding_platform'))
    .then((value) => {
      platform.value = value
    })
    .catch(() => {})
  try {
    localAvailable.value = await invoke<boolean>('embedded_infer_available')
    if (localAvailable.value === false) choose('cloud')
  } catch {
    localAvailable.value = false
    choose('cloud')
  }
})

function move(next: number) {
  step.value = next
  localStorage.setItem('rune-onboarding-step', String(next))
  message.value = ''
  if (next === 3) {
    void checkStarterMcp()
  }
}
function choose(value: Choice) {
  choice.value = value
  localStorage.setItem('rune-onboarding-choice', value)
  message.value = ''
}
async function closeSetup() {
  try {
    await getCurrentWindow().close()
  } catch (error) {
    message.value = String(error)
  }
}
async function checkSetup(): Promise<boolean> {
  busy.value = true
  message.value = ''
  try {
    const settings = await invoke<Settings>('get_settings')
    stopShortcut.value = settings.stopHotkey?.trim() || 'Escape'
    const local = settings.activeConnectionId === 'studio-embedded-infer'
    if ((choice.value === 'local') !== local) {
      message.value =
        choice.value === 'local'
          ? 'Select a local model below, then check again.'
          : 'Connect your model server below, then check again.'
      return false
    }
    const probe = await invoke<BackendProbe>('check_backend')
    if (
      probe.connectionId !== settings.activeConnectionId ||
      probe.connectionRevision !== settings.connectionsRevision
    ) {
      message.value = 'The connection changed while checking. Please check again.'
      return false
    }
    const model = (await invoke<string | null>('infer_agent_role'))?.trim() ?? ''
    if (!probe.status.reachable || !probe.status.authOk || probe.status.error) {
      message.value = probe.status.error || 'The selected model server is not ready yet.'
      return false
    } else if (!model || !probe.status.models?.some((entry) => entry.id === model)) {
      message.value = 'Select an available Agent model below, then check again.'
      return false
    } else {
      const selected = probe.status.models!.find((entry) => entry.id === model)!
      if (selected.imageIn === 'no' || selected.toolCalls === 'no') {
        message.value = 'Choose an Agent model that supports image input and tool calls.'
        return false
      }
      capabilityWarning.value =
        selected.imageIn === 'yes' && selected.toolCalls === 'yes'
          ? ''
          : 'The server has not confirmed this model’s screen and tool support. Your first task may not work.'
      move(3)
      return true
    }
  } catch (error) {
    message.value = String(error)
    return false
  } finally {
    busy.value = false
  }
}
async function proceed() {
  if (step.value === 2) return checkSetup()
  if (step.value < 3) return move(step.value + 1)
  if (!(await checkSetup())) return
  busy.value = true
  try {
    await invoke('onboarding_complete')
    await getCurrentWindow().close()
    localStorage.removeItem('rune-onboarding-step')
    localStorage.removeItem('rune-onboarding-choice')
    localStorage.removeItem('rune-onboarding-cloud-connection')
  } catch (error) {
    message.value = String(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <main class="setup-shell">
    <TitleBar title="Rune Studio" controls="close" bare :close-action="closeSetup" />
    <section v-if="step === 0" class="setup-center">
      <img :src="logo" alt="" class="setup-logo" />
      <p class="setup-tag">YOUR COMPUTER ASSISTANT</p>
      <h1>Meet Rune Studio</h1>
      <p>
        Ask Rune to help with a task. It can look at your screen, use your computer, and show you
        what it is doing.
      </p>
      <p v-if="platform === 'linux' || platform === 'macos'" role="status">
        Computer-use support on this platform is still in development. Some actions are unavailable.
      </p>
    </section>
    <section v-else-if="step === 1" class="setup-content">
      <div>
        <p class="setup-tag">YOUR MODEL</p>
        <h1>Choose how Rune thinks</h1>
        <p>Your choice becomes the active model connection. You can change it later in Settings.</p>
      </div>
      <div class="setup-choices">
        <RuneButton
          type="button"
          :disabled="localAvailable !== true"
          :aria-pressed="choice === 'local'"
          :class="{ selected: choice === 'local' }"
          @click="choose('local')"
        >
          <strong>On this computer</strong
          ><span>{{
            localAvailable === false
              ? 'Local setup is not available in this build.'
              : 'Download a model and run it with local Infer.'
          }}</span>
        </RuneButton>
        <RuneButton
          type="button"
          :aria-pressed="choice === 'cloud'"
          :class="{ selected: choice === 'cloud' }"
          @click="choose('cloud')"
        >
          <strong>Cloud or other server</strong
          ><span>Connect a compatible model server using its address and credentials.</span>
        </RuneButton>
      </div>
    </section>
    <section v-else-if="step === 2" class="setup-content">
      <div>
        <p class="setup-tag">GET READY</p>
        <h1>{{ choice === 'local' ? 'Prepare your local model' : 'Connect your model server' }}</h1>
        <p>
          {{
            choice === 'local'
              ? 'Choose and download a model. Rune will prepare it for you.'
              : 'Connect your server and choose its Agent model.'
          }}
        </p>
      </div>
      <OnboardingLocalModel v-if="choice === 'local'" @ready="checkSetup" />
      <OnboardingCloudModel v-else @ready="checkSetup" @cancel="move(1)" />
    </section>
    <section v-else class="setup-center step-try">
      <img :src="logo" alt="" class="setup-logo small" />
      <p class="setup-tag">READY TO TRY</p>
      <h1>Start with a small task</h1>
      <p>
        Rune captures your screen and stores task screenshots and details on this computer. Review
        its actions, approve requests when asked, and use <strong>{{ stopShortcut }}</strong> to
        stop.
      </p>

      <div v-for="starter in starters" :key="starter.name" class="mcp-starter-box">
        <div class="mcp-starter-info">
          <div class="mcp-starter-title">
            <strong>{{ starter.title }}</strong>
            <span v-if="mcpInstalled[starter.name]" class="mcp-badge success">Installed</span>
          </div>
          <span class="mcp-starter-desc">{{ starter.description }}</span>
        </div>
        <RuneButton
          v-if="!mcpInstalled[starter.name]"
          size="sm"
          variant="primary"
          :disabled="!!mcpBusy"
          class="mcp-install-btn"
          @click="installStarterMcp(starter)"
        >
          {{ mcpBusy === starter.name ? 'Installing…' : mcpFailed ? 'Try again' : starter.button }}
        </RuneButton>
      </div>
      <p
        v-if="mcpStatus"
        class="mcp-status"
        :class="{ failed: mcpFailed }"
        :role="mcpFailed ? 'alert' : 'status'"
      >
        {{ mcpStatus }}
      </p>

      <div class="setup-prompts-section">
        <p class="setup-prompts-title">SUGGESTED TEST PROMPTS (CLICK TO COPY)</p>
        <div class="setup-prompts-grid">
          <div
            v-for="item in testPrompts"
            :key="item.prompt"
            role="button"
            tabindex="0"
            class="setup-prompt-card"
            :class="{ copied: copiedPrompt === item.prompt }"
            @click="copyPrompt(item.prompt)"
            @keydown.enter="copyPrompt(item.prompt)"
            @keydown.space.prevent="copyPrompt(item.prompt)"
          >
            <div class="prompt-card-header">
              <span class="prompt-card-cat">{{ item.category }}</span>
              <span v-if="copiedPrompt === item.prompt" class="prompt-copied-pill">Copied!</span>
              <span v-else class="prompt-card-tag">{{ item.tag }}</span>
            </div>
            <strong class="prompt-card-text">“{{ item.prompt }}”</strong>
          </div>
        </div>
      </div>

      <p v-if="capabilityWarning" role="status">{{ capabilityWarning }}</p>
      <p v-if="platform === 'linux' || platform === 'macos'" role="status">
        Computer-use support on this platform is still in development. Some actions are unavailable.
      </p>
    </section>
    <p v-if="message" role="alert" class="setup-error">{{ message }}</p>
    <footer class="setup-footer">
      <div
        class="setup-progress"
        role="progressbar"
        :aria-valuenow="step + 1"
        aria-valuemin="1"
        aria-valuemax="4"
        :aria-label="`Step ${step + 1} of 4`"
      >
        <span v-for="n in 4" :key="n" :class="{ current: n === step + 1 }" />
      </div>
      <RuneButton v-if="step > 0" variant="ghost" :disabled="busy" @click="move(step - 1)"
        >Back</RuneButton
      >
      <RuneButton
        class="setup-button"
        variant="primary"
        type="button"
        :disabled="busy || (step === 1 && localAvailable === null)"
        @click="proceed"
      >
        {{ busy ? 'Checking…' : step === 2 ? 'Check setup' : nextLabel }}
      </RuneButton>
    </footer>
  </main>
</template>

<style scoped>
.setup-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-height: 0;
  overflow: hidden;
  color: var(--color-rune-ink);
  background: radial-gradient(circle at 50% 5%, #242529, #101114 65%);
  font-family: var(--font-sans);
}
.setup-center {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 32px;
}
.setup-center p {
  max-width: 570px;
}
.setup-logo {
  width: 140px;
  height: 140px;
  margin-bottom: 40px;
}
.setup-logo.small {
  width: 90px;
  height: 90px;
  margin-bottom: 28px;
}
.setup-shell h1 {
  font-size: clamp(30px, 4vw, 46px);
  margin: 10px 0 14px;
  line-height: 1.1;
}
.setup-shell p {
  color: var(--color-rune-text-2);
  font-family: var(--font-rune-reading);
  line-height: 1.5;
}
.setup-tag {
  color: var(--accent) !important;
  font-family: var(--font-sans) !important;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.13em;
}
.setup-content {
  overflow-x: hidden;
  flex: 1;
  min-height: 0;
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-template-rows: minmax(0, 1fr);
  align-content: center;
  align-items: center;
  gap: 60px;
  padding: 24px 8%;
}
.setup-content > * {
  min-width: 0;
}
.setup-choices {
  display: grid;
  gap: 14px;
}
.setup-choices button,
.setup-note {
  text-align: left;
  border: 1px solid var(--color-rune-raised);
  background: var(--color-rune-surface);
  color: var(--color-rune-ink);
  border-radius: 12px;
  padding: 24px;
}
.setup-choices button {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 10px;
  cursor: pointer;
  font-family: inherit;
  text-transform: none;
}
.setup-choices button.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}
.setup-choices span {
  color: var(--color-rune-text-2);
  font-family: var(--font-rune-reading);
  line-height: 1.4;
}
.setup-error {
  color: var(--color-error) !important;
  margin: 0 8% 12px;
}
.setup-footer {
  height: 66px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 20px;
  padding: 0 28px;
  border-top: 1px solid var(--color-rune-raised);
}
.setup-progress {
  display: flex;
  gap: 5px;
  margin-right: auto;
}
.setup-progress span {
  width: 17px;
  height: 4px;
  background: var(--color-rune-raised);
  border-radius: 5px;
}
.setup-progress span.current {
  width: 45px;
  background: var(--accent);
}
@media (max-width: 760px) {
  .setup-content {
    grid-template-columns: 1fr;
    grid-template-rows: auto minmax(0, 1fr);
    gap: 20px;
    padding: 24px;
  }
  .setup-logo {
    margin-bottom: 20px;
  }
}
.setup-center.step-try {
  justify-content: flex-start;
  min-height: 0;
  overflow-x: hidden;
  overflow-y: auto;
  padding: 16px 32px 24px;
}
.setup-center.step-try::before,
.setup-center.step-try::after {
  content: '';
  margin: auto;
}
.setup-center.step-try > * {
  max-width: 100%;
  overflow-wrap: anywhere;
}
.mcp-starter-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  width: 100%;
  max-width: 680px;
  margin: 16px 0 6px;
  padding: 12px 18px;
  background: var(--color-rune-surface);
  border: 1px solid var(--color-rune-raised);
  border-radius: 10px;
  text-align: left;
}
.mcp-starter-info {
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.mcp-starter-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 14px;
  color: var(--color-rune-ink);
}
.mcp-starter-desc {
  font-size: 12px;
  color: var(--color-rune-text-2);
}
.mcp-starter-desc code {
  color: var(--accent);
  background: rgba(255, 255, 255, 0.06);
  padding: 2px 5px;
  border-radius: 4px;
}
.mcp-badge {
  font-size: 11px;
  font-weight: 600;
  padding: 2px 8px;
  border-radius: 4px;
}
.mcp-badge.success {
  background: rgba(46, 204, 113, 0.15);
  color: #2ecc71;
  border: 1px solid rgba(46, 204, 113, 0.3);
}
.mcp-install-btn {
  flex-shrink: 0;
}
.mcp-status {
  font-size: 12px;
  color: var(--accent);
  margin: 4px 0 8px;
}
.mcp-status.failed {
  color: var(--color-error);
}
.setup-prompts-section {
  width: 100%;
  max-width: 680px;
  margin-top: 12px;
  text-align: left;
}
.setup-prompts-title {
  font-size: 11px !important;
  font-weight: 700;
  letter-spacing: 0.1em;
  color: var(--color-rune-text-3) !important;
  margin-bottom: 8px;
}
.setup-prompts-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.setup-prompt-card {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  background: var(--color-rune-surface);
  border: 1px solid var(--color-rune-raised);
  border-radius: 8px;
  text-align: left;
  cursor: pointer;
  transition:
    border-color 0.15s,
    background-color 0.15s;
  font-family: inherit;
}
.setup-prompt-card:hover {
  border-color: var(--accent);
  background: rgba(255, 255, 255, 0.03);
}
.setup-prompt-card.copied {
  border-color: #2ecc71;
}
.prompt-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}
.prompt-card-cat {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
}
.prompt-card-tag {
  font-size: 10px;
  color: var(--color-rune-text-3);
  background: rgba(255, 255, 255, 0.05);
  padding: 1px 6px;
  border-radius: 4px;
}
.prompt-copied-pill {
  font-size: 10px;
  font-weight: 600;
  color: #2ecc71;
  background: rgba(46, 204, 113, 0.15);
  padding: 1px 6px;
  border-radius: 4px;
}
.prompt-card-text {
  font-size: 12px;
  line-height: 1.4;
  color: var(--color-rune-ink);
  font-weight: 500;
}
@media (max-width: 600px) {
  .setup-prompts-grid {
    grid-template-columns: 1fr;
  }
}
</style>