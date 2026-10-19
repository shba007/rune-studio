<script setup lang="ts">
import { useEventListener, usePreferredReducedMotion } from '@vueuse/core'
import { computed, ref } from 'vue'

import { useAnalysis } from '@/composables/useAnalysis'

import AnalysisFindings from './AnalysisFindings.vue'
import AnalysisStage from './AnalysisStage.vue'
import AppIcon from './AppIcon.vue'

/** The screen analysis window: the screenshot with numbered regions, and what supports each. */
const { snapshot, notice, busy, capture, ask, cancel } = useAnalysis()
const reducedMotion = usePreferredReducedMotion()

const question = ref('')
const hovered = ref<number | null>(null)
const pinned = ref<number | null>(null)
const active = computed(() => hovered.value ?? pinned.value)

function select(n: number) {
  pinned.value = pinned.value === n ? null : n
  document.getElementById(`finding-${n}`)?.scrollIntoView({
    block: 'nearest',
    behavior: reducedMotion.value === 'reduce' ? 'auto' : 'smooth',
  })
}

useEventListener(window, 'keydown', (event: KeyboardEvent) => {
  if (event.key === 'Escape') {
    hovered.value = null
    pinned.value = null
  }
})

function submit() {
  const text = question.value.trim()
  if (busy.value) return
  void (snapshot.value.image ? ask(text) : capture(text))
}

function newCapture() {
  if (busy.value) return
  pinned.value = null
  void capture(question.value.trim())
}

const statusLine = computed(() => {
  switch (snapshot.value.status) {
    case 'capturing':
      return 'Looking at your screen'
    case 'asking':
      return 'Reading what is on it'
    case 'error':
      return snapshot.value.error ?? 'The analysis failed'
    case 'cancelled':
      return 'Cancelled'
    case 'done': {
      const count = snapshot.value.findings.length
      const seconds = (snapshot.value.latencyMs / 1000).toFixed(1)
      return `${count} ${count === 1 ? 'finding' : 'findings'} in ${seconds} s`
    }
    default:
      return 'Nothing analysed yet'
  }
})

const when = computed(() => {
  const date = new Date(snapshot.value.createdUtc)
  return Number.isNaN(date.getTime()) ? '' : date.toLocaleTimeString()
})

const button =
  'inline-flex h-8 cursor-pointer items-center gap-2 rounded-full border border-black/20 px-3 text-label uppercase outline-none transition hover:bg-black/6 focus-visible:ring-2 focus-visible:ring-current disabled:cursor-default disabled:opacity-40 dark:border-white/20 dark:hover:bg-white/10'
</script>

<template>
  <div class="mx-auto flex min-h-full max-w-[1400px] flex-col gap-4 p-4">
    <header
      class="flex items-baseline justify-between gap-4 border-b border-dotted border-current/40 pb-2 text-caption uppercase"
    >
      <h1 class="m-0 text-caption">01 / Screen analysis</h1>
      <p class="m-0 truncate opacity-70">
        {{ [snapshot.model, when].filter(Boolean).join('  ·  ') }}
      </p>
    </header>

    <div class="grid flex-1 gap-4 min-[900px]:grid-cols-[minmax(0,1fr)_320px]">
      <section class="flex min-w-0 flex-col gap-3" aria-label="Screenshot">
        <AnalysisStage
          v-if="snapshot.image"
          :image="snapshot.image"
          :width="snapshot.width"
          :height="snapshot.height"
          :findings="snapshot.findings"
          :active="active"
          @hover="hovered = $event"
          @select="select"
        />
        <div
          v-else
          class="grid aspect-video w-full place-items-center rounded-lg border border-dashed border-current/30 text-body opacity-70"
        >
          {{ busy ? statusLine : 'Capture the screen to analyse it' }}
        </div>

        <div class="flex items-center gap-3" role="status" aria-live="polite">
          <p
            class="m-0 min-w-0 flex-1 text-status"
            :class="snapshot.status === 'error' ? 'text-[#d71921]' : 'opacity-80'"
          >
            {{ statusLine }}
          </p>
          <button v-if="busy" type="button" :class="button" @click="cancel()">
            <AppIcon name="close" size="sm" /> Cancel
          </button>
          <button
            v-else-if="snapshot.status === 'error'"
            type="button"
            :class="button"
            @click="submit()"
          >
            <AppIcon name="refresh" size="sm" /> Retry
          </button>
        </div>
        <div v-if="busy" class="h-0.5 overflow-hidden rounded bg-current/15" aria-hidden="true">
          <div class="h-full w-1/3 rounded bg-current/60 motion-safe:animate-pulse" />
        </div>
        <p v-if="notice" class="m-0 text-status text-[#d71921]">{{ notice }}</p>

        <article v-if="snapshot.status === 'done'" class="flex flex-col gap-1">
          <h2 class="m-0 text-caption uppercase opacity-60">
            {{ snapshot.question }}
          </h2>
          <p class="m-0 text-body whitespace-pre-line">
            {{ snapshot.answer || 'No answer was given.' }}
          </p>
        </article>

        <form class="flex items-center gap-2" @submit.prevent="submit">
          <input
            v-model="question"
            type="text"
            class="h-8 min-w-0 flex-1 rounded-full border border-black/20 bg-transparent px-3 text-body outline-none placeholder:opacity-50 focus-visible:ring-2 focus-visible:ring-current dark:border-white/20"
            placeholder="Ask about this screen"
            aria-label="Ask about this screen"
          />
          <button type="submit" :class="button" :disabled="busy" aria-label="Ask">
            <AppIcon name="send" size="sm" /> Ask
          </button>
          <button type="button" :class="button" :disabled="busy" @click="newCapture()">
            <AppIcon name="scan" size="sm" /> New capture
          </button>
        </form>
      </section>

      <aside class="min-w-0" aria-label="Evidence">
        <h2 class="mb-2 mt-0 text-caption uppercase opacity-60">Evidence</h2>
        <AnalysisFindings
          v-if="snapshot.findings.length"
          :findings="snapshot.findings"
          :active="active"
          @hover="hovered = $event"
          @select="select"
        />
        <p v-else class="m-0 text-body opacity-60">
          {{
            snapshot.status === 'done'
              ? 'The model pointed at nothing in particular.'
              : 'Findings appear here.'
          }}
        </p>
      </aside>
    </div>
  </div>
</template>