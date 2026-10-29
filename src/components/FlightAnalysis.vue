<script setup lang="ts">
import ModelIcon from './ModelIcon.vue'
import { computed, ref, watch } from 'vue'
import { useElementSize } from '@vueuse/core'
import { invoke } from '@tauri-apps/api/core'
import type { FlightAnalysis } from '@/config/flight'
import { EMPTY_SNAPSHOT } from '@/config/analysis'
import type { AnalysisGroup, VisualAnalysisVisibility } from '@/types/protocol'
import { fileUrl } from '@/composables/useFlight'
import AnalysisLayer from './AnalysisLayer.vue'
import WhenText from './WhenText.vue'

const props = defineProps<{
  id: string
  analysis?: FlightAnalysis | null
  summary: string | null
}>()
const capture = ref<string | null>(null)
const loading = ref(false)
const missing = ref(false)
const answer = computed(() => props.analysis?.answer || props.summary)
const image = ref<HTMLImageElement | null>(null)
const { width, height } = useElementSize(image)
const groups: { key: AnalysisGroup; label: string }[] = [
  { key: 'entities', label: 'Entities' },
  { key: 'regions', label: 'Regions & Context' },
  { key: 'structure', label: 'Structure & Landmarks' },
  { key: 'depth', label: 'Depth' },
]
const visibleGroups = ref<VisualAnalysisVisibility>({
  entities: true,
  regions: true,
  structure: true,
  depth: true,
})
const snapshot = computed(() => ({
  ...EMPTY_SNAPSHOT,
  id: `replay-${props.id}`,
  findings: props.analysis?.findings ?? [],
  visibleGroups: visibleGroups.value,
}))

watch(
  [() => props.id, () => props.analysis?.captureFile],
  async ([id, relative], _, onCleanup) => {
    let current = true
    onCleanup(() => {
      current = false
    })
    capture.value = null
    missing.value = false
    loading.value = Boolean(relative)
    if (!relative) return
    try {
      const path = await invoke<string>('flight_file', { id, relative })
      if (current) capture.value = fileUrl(path)
    } catch {
      if (current) missing.value = true
    } finally {
      if (current) loading.value = false
    }
  },
  { immediate: true },
)
</script>

<template>
  <section
    class="flex min-h-0 flex-1 flex-col overflow-auto md:flex-row"
    aria-label="Visual analysis task"
  >
    <figure class="m-0 flex min-h-64 min-w-0 flex-1 flex-col bg-rune-sunken p-4">
      <div class="flex min-h-0 flex-1 items-center justify-center">
        <div v-if="capture && !missing" class="relative max-w-full leading-none">
          <img
            ref="image"
            v-if="capture && !missing"
            :key="id"
            :src="capture"
            alt="Saved visual analysis capture"
            class="block max-h-[65vh] max-w-full object-contain"
            @error="missing = true"
          />
          <AnalysisLayer
            v-if="width > 0 && height > 0"
            :snapshot="snapshot"
            :view="{ width, height, scaleFactor: 1 }"
            :scan-key="0"
          />
        </div>
        <p
          v-else
          class="m-0 p-6 text-rune-body text-rune-text-2"
          data-testid="analysis-capture-notice"
        >
          {{
            loading
              ? 'Loading capture…'
              : missing
                ? 'The saved capture is unavailable. It may have been deleted or moved.'
                : 'No capture was saved for this task. Older in-memory images cannot be restored.'
          }}
        </p>
      </div>
      <figcaption
        v-if="analysis"
        class="mt-3 flex flex-wrap gap-x-3 text-rune-caption text-rune-text-2"
      >
        <span>Captured <WhenText :iso="analysis.capturedUtc" /></span>
        <span v-if="analysis.model" class="inline-flex items-center gap-2"
          ><ModelIcon :model="analysis.model" />{{ analysis.model }}</span
        >
      </figcaption>
    </figure>
    <aside class="w-full shrink-0 overflow-y-auto p-5 md:w-80">
      <fieldset v-if="analysis?.findings.length" class="mb-5 p-0">
        <legend class="mb-2 text-rune-heading text-rune-text-2 uppercase">Highlights</legend>
        <label
          v-for="group in groups"
          :key="group.key"
          class="flex items-center gap-2 py-1 text-rune-caption text-rune-text-2"
        >
          <input v-model="visibleGroups[group.key]" type="checkbox" :aria-label="group.label" />
          {{ group.label }}
        </label>
      </fieldset>
      <h2 class="m-0 text-rune-heading text-rune-text-2 uppercase">Answer</h2>
      <p class="whitespace-pre-wrap text-rune-body text-rune-ink" data-testid="analysis-answer">
        {{ answer || 'No answer was recorded.' }}
      </p>
      <p v-if="analysis?.error" class="text-rune-caption text-rune-text-2">{{ analysis.error }}</p>
      <h2
        v-if="analysis?.findings.length"
        class="mt-6 mb-2 text-rune-heading text-rune-text-2 uppercase"
      >
        Observations
      </h2>
      <article v-for="finding in analysis?.findings ?? []" :key="finding.n" class="py-3">
        <h3 class="m-0 text-rune-body text-rune-ink">{{ finding.n }} · {{ finding.label }}</h3>
        <p
          v-if="finding.pose?.rejected"
          role="status"
          class="my-1 text-rune-caption text-amber-300"
        >
          Some pose data was invalid and omitted. Valid landmarks and connections are shown.
        </p>
        <p class="my-1 text-rune-caption text-rune-text-2">{{ finding.description }}</p>
        <p class="m-0 text-rune-caption text-rune-text-2">{{ finding.evidence }}</p>
      </article>
      <p class="mt-5 text-rune-caption text-rune-text-2">Model observations, not verified facts.</p>
    </aside>
  </section>
</template>