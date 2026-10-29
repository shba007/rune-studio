<script setup lang="ts">
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import RuneButton from './RuneButton.vue'
import { computed, ref } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { CaptureReport, ClickReport } from '@/types/protocol'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const x = ref(500)
const y = ref(500)
const text = ref('hello')
const delaySeconds = ref(3)

const busy = ref<'capture' | 'click' | 'move' | 'type' | null>(null)
const capture = ref<CaptureReport | null>(null)
const click = ref<ClickReport | null>(null)

const delayMs = computed(() => Math.max(0, delaySeconds.value) * 1000)

const captureSummary = computed(() => {
  const c = capture.value
  if (!c) return ''
  const kb = Math.round(c.bytes / 1024)
  const scale = Math.round(c.monitor.scaleFactor * 100)
  return `Native preview ${c.monitor.width}×${c.monitor.height} · Model input ${c.width}×${c.height}, ${kb} KB · ${c.millis} ms · Display scale ${scale}%`
})

async function doCapture() {
  busy.value = 'capture'
  capture.value = (await agent.debugCapture()) ?? null
  busy.value = null
}

async function doClick() {
  busy.value = 'click'
  click.value = null
  click.value = (await agent.debugClick({ x: x.value, y: y.value, delayMs: delayMs.value })) ?? null
  busy.value = null
}

async function doMove() {
  busy.value = 'move'
  await agent.debugMove(x.value, y.value, delayMs.value)
  busy.value = null
}

async function doType() {
  busy.value = 'type'
  await agent.debugType(text.value, delayMs.value)
  busy.value = null
}
</script>

<template>
  <HubGroup
    title="Eyes and hands"
    :hint="`Stop interrupts any of these: ${agent.settings.value?.stopHotkey || 'Escape'} or the tray.`"
  >
    <HubRow label="Capture screen" hint="Take one capture at native resolution.">
      <RuneButton size="sm" :disabled="busy !== null" @click="doCapture">
        {{ busy === 'capture' ? 'Capturing…' : 'Capture screen' }}
      </RuneButton>
    </HubRow>
    <HubRow label="Wait before acting" hint="Seconds before a click, move or type.">
      <input
        v-model.number="delaySeconds"
        type="number"
        min="0"
        max="30"
        aria-label="Wait before acting (seconds)"
        class="h-10 w-20 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink"
      />
    </HubRow>
    <HubRow label="Cursor position" hint="X and Y from 0 to 1000.">
      <input
        v-model.number="x"
        type="number"
        min="0"
        max="1000"
        aria-label="X (0 to 1000)"
        class="h-10 w-20 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink"
      />
      <input
        v-model.number="y"
        type="number"
        min="0"
        max="1000"
        aria-label="Y (0 to 1000)"
        class="h-10 w-20 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink"
      />
    </HubRow>
    <HubRow label="Cursor actions" hint="Act at the position above.">
      <RuneButton size="sm" :disabled="busy !== null" @click="doMove">
        {{ busy === 'move' ? 'Moving…' : 'Move cursor' }}
      </RuneButton>
      <RuneButton size="sm" :disabled="busy !== null" @click="doClick">
        {{ busy === 'click' ? 'Clicking…' : 'Click' }}
      </RuneButton>
    </HubRow>
    <HubRow label="Type text" hint="Sent to the focused window.">
      <input
        v-model="text"
        type="text"
        aria-label="Text to type"
        class="h-10 w-48 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink"
        @keydown.enter="doType"
      />
      <RuneButton size="sm" :disabled="busy !== null" @click="doType">
        {{ busy === 'type' ? 'Typing…' : 'Type' }}
      </RuneButton>
    </HubRow>
    <template #after>
      <img
        v-if="capture"
        :src="capture.thumbnail"
        alt="Captured screen at native resolution"
        class="w-full rounded-rune-tile bg-rune-raised"
      />
      <p v-if="capture" class="m-0 px-rune-2 text-rune-caption text-rune-text-2">
        {{ captureSummary }}
      </p>
      <p v-if="click" class="m-0 px-rune-2 text-rune-caption text-rune-text-2">
        pixel {{ click.pixelX }}, {{ click.pixelY }} · cursor {{ click.cursorX }},
        {{ click.cursorY }}
      </p>
    </template>
  </HubGroup>
</template>