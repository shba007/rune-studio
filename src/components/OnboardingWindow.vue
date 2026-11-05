<script setup lang="ts">
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { BackendProbe, Settings } from '@/types/protocol'

type Choice = 'local' | 'cloud'
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
const nextLabel = computed(() => (step.value === 3 ? 'Open Rune' : 'Continue'))

function move(next: number) {
  step.value = next
  localStorage.setItem('rune-onboarding-step', String(next))
  message.value = ''
}
function choose(value: Choice) {
  choice.value = value
  localStorage.setItem('rune-onboarding-choice', value)
  message.value = ''
}
async function openSetup() {
  try {
    await invoke('open_settings_window', { section: 'models' })
  } catch (error) {
    message.value = String(error)
  }
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
    const local = settings.activeConnectionId === 'studio-embedded-infer'
    if ((choice.value === 'local') !== local) {
      message.value =
        choice.value === 'local'
          ? 'Select Studio local Infer in Models & tools, then check again.'
          : 'Add and activate your cloud or other model server in Models & tools, then check again.'
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
    const model = settings.models.agent.trim()
    if (!probe.status.reachable || !probe.status.authOk || probe.status.error) {
      message.value = probe.status.error || 'The selected model server is not ready yet.'
      return false
    } else if (!model || !probe.status.models?.some((entry) => entry.id === model)) {
      message.value = 'Select an available Agent model in Models & tools, then check again.'
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
  } catch (error) {
    message.value = String(error)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <main class="setup-shell">
    <header data-tauri-drag-region class="setup-header">
      <span>RUNE STUDIO</span>
      <button type="button" aria-label="Close setup" @click="closeSetup">×</button>
    </header>
    <section v-if="step === 0" class="setup-center">
      <img :src="logo" alt="" class="setup-logo" />
      <p class="setup-tag">YOUR COMPUTER ASSISTANT</p>
      <h1>Meet Rune Studio</h1>
      <p>
        Ask Rune to help with a task. It can look at your screen, use your computer, and show you
        what it is doing.
      </p>
    </section>
    <section v-else-if="step === 1" class="setup-content">
      <div>
        <p class="setup-tag">YOUR MODEL</p>
        <h1>Choose how Rune thinks</h1>
        <p>Your choice becomes the active model connection. You can change it later in Settings.</p>
      </div>
      <div class="setup-choices">
        <button
          type="button"
          :aria-pressed="choice === 'local'"
          :class="{ selected: choice === 'local' }"
          @click="choose('local')"
        >
          <strong>On this computer</strong
          ><span>Download a model and run it with local Infer.</span>
        </button>
        <button
          type="button"
          :aria-pressed="choice === 'cloud'"
          :class="{ selected: choice === 'cloud' }"
          @click="choose('cloud')"
        >
          <strong>Cloud or other server</strong
          ><span>Connect a compatible model server using its address and credentials.</span>
        </button>
      </div>
    </section>
    <section v-else-if="step === 2" class="setup-content">
      <div>
        <p class="setup-tag">GET READY</p>
        <h1>{{ choice === 'local' ? 'Prepare your local model' : 'Connect your model server' }}</h1>
        <p>
          {{
            choice === 'local'
              ? 'Choose and download a model, then select Use this model.'
              : 'Add your server, activate the connection, and choose its Agent model.'
          }}
        </p>
        <button class="setup-button" type="button" @click="openSetup">Open Models & tools</button>
      </div>
      <div class="setup-note">
        <strong>Close Settings and come back when you are ready.</strong>
        <p>Rune will check the connection and selected model before continuing.</p>
        <p v-if="choice === 'cloud'">
          Screen content and task data will be sent to the server you choose.
        </p>
      </div>
    </section>
    <section v-else class="setup-center">
      <img :src="logo" alt="" class="setup-logo small" />
      <p class="setup-tag">READY TO TRY</p>
      <h1>Start with a small task</h1>
      <p>Open the Rune composer and try: <strong>“Open Notepad and type hello.”</strong></p>
      <p>
        Rune captures your screen and stores task screenshots and details on this computer. Review
        its actions, approve requests when asked, and use <strong>Ctrl+Alt+Esc</strong> to stop.
      </p>
      <p v-if="capabilityWarning" role="status">{{ capabilityWarning }}</p>
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
      <button v-if="step > 0" type="button" :disabled="busy" @click="move(step - 1)">Back</button>
      <button class="setup-button" type="button" :disabled="busy" @click="proceed">
        {{ busy ? 'Checking…' : step === 2 ? 'Check setup' : nextLabel }}
      </button>
    </footer>
  </main>
</template>

<style scoped>
.setup-shell {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
  color: var(--color-rune-ink);
  background: radial-gradient(circle at 50% 5%, #242529, #101114 65%);
  font-family: var(--font-sans);
}
.setup-header {
  height: 54px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 26px;
  color: var(--color-rune-text-3);
  font-size: 12px;
  letter-spacing: 0.12em;
}
.setup-header button,
.setup-footer button:not(.setup-button) {
  border: 0;
  color: var(--color-rune-text-2);
  background: transparent;
  cursor: pointer;
  font-size: 18px;
  font-family: inherit;
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
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-content: center;
  align-items: center;
  gap: 60px;
  padding: 50px 8%;
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
  gap: 10px;
  cursor: pointer;
  font-family: inherit;
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
.setup-button {
  border: 0;
  border-radius: 7px;
  background: var(--color-rune-ink);
  color: #171719;
  padding: 10px 18px;
  font-weight: 650;
  cursor: pointer;
  font-family: inherit;
}
.setup-button:disabled {
  opacity: 0.5;
  cursor: default;
}
.setup-error {
  color: var(--color-error) !important;
  margin: 0 8% 12px;
}
.setup-footer {
  height: 66px;
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
    gap: 20px;
    padding: 24px;
  }
  .setup-logo {
    margin-bottom: 20px;
  }
}
</style>