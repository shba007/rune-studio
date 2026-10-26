<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { FlightAnalysis } from '@/config/flight'
import { fileUrl } from '@/composables/useFlight'
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
    aria-label="Screen analysis session"
  >
    <figure class="m-0 flex min-h-64 min-w-0 flex-1 flex-col bg-[#0a0a0a] p-4">
      <div class="flex min-h-0 flex-1 items-center justify-center">
        <img
          v-if="capture && !missing"
          :key="id"
          :src="capture"
          alt="Saved screen analysis capture"
          class="max-h-full max-w-full object-contain"
          @error="missing = true"
        />
        <p v-else class="m-0 p-6 text-body text-zinc-400" data-testid="analysis-capture-notice">
          {{
            loading
              ? 'Loading capture…'
              : missing
                ? 'The saved capture is unavailable. It may have been deleted or moved.'
                : 'No capture was saved for this session. Older in-memory images cannot be restored.'
          }}
        </p>
      </div>
      <figcaption v-if="analysis" class="mt-3 flex flex-wrap gap-x-3 text-label text-zinc-500">
        <span>Captured <WhenText :iso="analysis.capturedUtc" /></span>
        <span v-if="analysis.model">{{ analysis.model }}</span>
      </figcaption>
    </figure>
    <aside
      class="w-full shrink-0 overflow-y-auto border-t border-white/10 p-5 md:w-80 md:border-t-0 md:border-l"
    >
      <h2 class="m-0 text-caption text-zinc-500 uppercase">Answer</h2>
      <p class="whitespace-pre-wrap text-body text-zinc-100" data-testid="analysis-answer">
        {{ answer || 'No answer was recorded.' }}
      </p>
      <p v-if="analysis?.error" class="text-label text-zinc-400">{{ analysis.error }}</p>
      <h2 v-if="analysis?.findings.length" class="mt-6 mb-2 text-caption text-zinc-500 uppercase">
        Observations
      </h2>
      <article
        v-for="finding in analysis?.findings ?? []"
        :key="finding.n"
        class="border-t border-white/8 py-3"
      >
        <h3 class="m-0 text-body text-zinc-100">{{ finding.n }} · {{ finding.label }}</h3>
        <p class="my-1 text-label text-zinc-400">{{ finding.description }}</p>
        <p class="m-0 text-label text-zinc-500">{{ finding.evidence }}</p>
      </article>
      <p class="mt-5 text-label text-zinc-500">Model observations, not verified facts.</p>
    </aside>
  </section>
</template>