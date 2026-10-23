<script setup lang="ts">
import { computed, ref } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { CaptureReport, ClickReport } from '@/types/protocol'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const FIELD =
  'min-w-0 rounded-lg bg-white/8 px-2 py-1 text-label text-zinc-100 outline-none focus:ring-1 focus:ring-white/30'
const BUTTON =
  'cursor-pointer rounded-full bg-white/8 px-3 py-1 text-label text-zinc-200 transition hover:bg-white/15'

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
  return `${c.width}×${c.height} · ${kb} KB · ${c.millis} ms from ${c.monitor.width}×${c.monitor.height} at ${scale}%`
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
  <div class="flex flex-col gap-2 border-t border-white/8 pt-4">
    <h4 class="text-caption text-zinc-500 uppercase">Eyes and hands</h4>

    <div class="flex flex-col gap-2">
      <button
        type="button"
        :class="[BUTTON, 'self-start']"
        :disabled="busy !== null"
        @click="doCapture"
      >
        {{ busy === 'capture' ? 'Capturing…' : 'Capture screen' }}
      </button>
      <img
        v-if="capture"
        :src="capture.thumbnail"
        alt="What the model would see"
        class="w-full rounded-lg border border-white/8"
      />
      <p v-if="capture" class="text-label text-zinc-500">{{ captureSummary }}</p>
    </div>

    <label class="flex items-center gap-2 text-label text-zinc-500">
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
      <button type="button" :class="BUTTON" :disabled="busy !== null" @click="doClick">
        {{ busy === 'click' ? 'Clicking…' : 'Click' }}
      </button>
    </div>
    <button type="button" :class="[BUTTON, 'self-start']" :disabled="busy !== null" @click="doMove">
      {{ busy === 'move' ? 'Moving…' : 'Move cursor' }}
    </button>
    <p v-if="click" class="text-label text-zinc-500">
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
      <button type="button" :class="BUTTON" :disabled="busy !== null" @click="doType">
        {{ busy === 'type' ? 'Typing…' : 'Type' }}
      </button>
    </div>
    <p class="text-label text-zinc-600">Stop interrupts any of these: Ctrl+Alt+Esc or the tray.</p>
  </div>
</template>