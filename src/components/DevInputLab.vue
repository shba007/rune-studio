<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { computed, ref } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { CaptureReport, ClickReport } from '@/types/protocol'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const FIELD = 'min-w-0 rounded-rune-tile bg-rune-surface px-2 py-1 text-rune-caption text-rune-ink'
const BUTTON =
  'cursor-pointer rounded-full bg-rune-surface px-3 py-1 text-rune-caption text-rune-ink transition hover:bg-white/15'

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
  <div class="flex flex-col gap-2 pt-4">
    <h4 class="text-rune-heading text-rune-text-2 uppercase">Eyes and hands</h4>

    <div class="flex flex-col gap-2">
      <RuneButton size="sm" class="self-start" :disabled="busy !== null" @click="doCapture">
        {{ busy === 'capture' ? 'Capturing…' : 'Capture screen' }}
      </RuneButton>
      <img
        v-if="capture"
        :src="capture.thumbnail"
        alt="Captured screen at native resolution"
        class="w-full rounded-rune-tile bg-rune-raised"
      />
      <p v-if="capture" class="text-rune-caption text-rune-text-2">{{ captureSummary }}</p>
    </div>

    <label class="flex items-center gap-2 text-rune-caption text-rune-text-2">
      Wait before acting
      <input
        v-model.number="delaySeconds"
        type="number"
        min="0"
        max="30"
        :class="[FIELD, 'w-14']"
      />
      s
    </label>

    <div class="flex items-center gap-2">
      <input
        v-model.number="x"
        type="number"
        min="0"
        max="1000"
        aria-label="X (0 to 1000)"
        :class="[FIELD, 'w-16']"
      />
      <input
        v-model.number="y"
        type="number"
        min="0"
        max="1000"
        aria-label="Y (0 to 1000)"
        :class="[FIELD, 'w-16']"
      />
      <RuneButton size="sm" :disabled="busy !== null" @click="doClick">
        {{ busy === 'click' ? 'Clicking…' : 'Click' }}
      </RuneButton>
    </div>
    <RuneButton size="sm" class="self-start" :disabled="busy !== null" @click="doMove">
      {{ busy === 'move' ? 'Moving…' : 'Move cursor' }}
    </RuneButton>
    <p v-if="click" class="text-rune-caption text-rune-text-2">
      pixel {{ click.pixelX }}, {{ click.pixelY }} · cursor {{ click.cursorX }}, {{ click.cursorY }}
    </p>

    <div class="flex items-center gap-2">
      <input
        v-model="text"
        type="text"
        aria-label="Text to type"
        :class="[FIELD, 'flex-1']"
        @keydown.enter="doType"
      />
      <RuneButton size="sm" :disabled="busy !== null" @click="doType">
        {{ busy === 'type' ? 'Typing…' : 'Type' }}
      </RuneButton>
    </div>
    <p class="text-rune-caption text-rune-text-3">
      Stop interrupts any of these: {{ agent.settings.value?.stopHotkey || 'Escape' }} or the tray.
    </p>
  </div>
</template>