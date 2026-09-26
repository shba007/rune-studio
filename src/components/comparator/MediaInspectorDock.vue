<script setup lang="ts">
import { computed } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useMediaStore } from '@/stores/mediaStore'
import { useAutoPipeline } from '@/composables/useAutoPipeline'
import { useMediaProfiler } from '@/composables/useMediaProfiler'

const mediaStore = useMediaStore()
const { cancelPipeline } = useAutoPipeline()
const { purgeArtifact } = useMediaProfiler()

async function handlePurgeSelected() {
  if (!mediaStore.selectedMedia) return
  await purgeArtifact(mediaStore.selectedMedia)
}

const isProcessing = computed(
  () => mediaStore.pipelineState === 'profiling' || mediaStore.pipelineState === 'clustering',
)

function formatBytes(bytes: number): string {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function formatDate(ms: number): string {
  if (!ms) return 'N/A'
  return new Date(ms).toLocaleString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

const assessment = computed(() => mediaStore.selectedMediaArtifact?.assessment)
const fidelityScore = computed(() => mediaStore.selectedMediaArtifact?.fidelityScore ?? null)

const fidelityDisplay = computed(() => {
  if (fidelityScore.value === null) return null
  const score = fidelityScore.value
  if (score >= 8.0)
    return { label: 'Optimal Anchor Candidate', color: 'bg-emerald-500', text: 'text-emerald-400' }
  if (score >= 5.5)
    return { label: 'Acceptable Quality', color: 'bg-amber-500', text: 'text-amber-400' }
  return { label: 'Low Quality / Partial Occlusion', color: 'bg-red-500', text: 'text-red-400' }
})
</script>

<template>
  <aside
    class="w-[420px] h-full border-l border-[#1f1f26] bg-[#101015] flex flex-col overflow-y-auto shrink-0 select-none"
  >
    <div
      class="px-4 py-3.5 border-b border-[#1f1f26] flex items-center justify-between bg-[#121217]"
    >
      <h2
        class="text-xs font-semibold uppercase tracking-wider text-neutral-400 flex items-center gap-2"
      >
        <span v-if="isProcessing" class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
        <span>{{ isProcessing ? 'Live Arbitration Pipeline' : 'Media Inspector' }}</span>
      </h2>
      <div v-if="mediaStore.selectedMedia && !isProcessing" class="flex items-center gap-2">
        <button
          type="button"
          @click="handlePurgeSelected"
          title="Clear assessment cache for this asset"
          class="text-xs text-amber-500/80 hover:text-amber-400 px-2 py-0.5 rounded-lg border border-amber-900/40 hover:bg-amber-950/40 transition-colors"
        >
          Clear Cache
        </button>
        <button
          @click="mediaStore.clearSelection()"
          class="text-xs text-neutral-500 hover:text-neutral-300 transition-colors"
        >
          Close
        </button>
      </div>
    </div>

    <!-- 1. LIVE MONITOR -->
    <div v-if="isProcessing" class="p-4 flex flex-col gap-4">
      <div
        class="p-3 bg-emerald-950/30 border border-emerald-800/60 rounded-2xl flex items-center justify-between"
      >
        <span class="text-emerald-400 font-medium text-xs flex items-center gap-2">
          <svg class="w-3.5 h-3.5 animate-spin shrink-0" fill="none" viewBox="0 0 24 24">
            <circle
              class="opacity-25"
              cx="12"
              cy="12"
              r="10"
              stroke="currentColor"
              stroke-width="4"
            />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8H4z" />
          </svg>
          {{ mediaStore.pipelineProgress.currentStage || 'Processing...' }}
        </span>
        <span class="font-mono text-neutral-400 text-xs font-semibold">
          {{ mediaStore.pipelineProgress.current }} / {{ mediaStore.pipelineProgress.total }}
        </span>
      </div>

      <div class="flex flex-col gap-1.5">
        <span class="text-xs text-neutral-400 font-medium">Analyzing Asset:</span>
        <div
          class="aspect-video bg-[#14141a] border border-[#242430] rounded-2xl overflow-hidden flex items-center justify-center relative shadow-inner"
        >
          <template v-if="mediaStore.pipelineProgress.currentItem">
            <img
              :src="mediaStore.pipelineProgress.currentItem.thumbnailUrl"
              loading="lazy"
              decoding="async"
              class="w-full h-full object-contain"
            />
          </template>
          <span v-else class="text-xs text-neutral-600">Awaiting stream...</span>
        </div>
        <span
          class="text-xs font-mono text-neutral-300 truncate"
          :title="mediaStore.pipelineProgress.currentItem?.name"
        >
          {{ mediaStore.pipelineProgress.currentItem?.name || 'Processing...' }}
        </span>
      </div>

      <button
        type="button"
        @click="cancelPipeline"
        class="w-full py-2 bg-red-950/80 hover:bg-red-900 border border-red-800 text-red-200 text-xs font-medium rounded-xl transition-all"
      >
        Halt Pipeline
      </button>
    </div>

    <!-- 2. SELECTED ASSET INSPECTOR -->
    <div v-else-if="mediaStore.selectedMedia" class="p-4 flex flex-col gap-4">
      <div
        class="aspect-video bg-[#14141a] border border-[#242430] rounded-2xl overflow-hidden flex items-center justify-center relative shadow-lg"
      >
        <img
          v-if="mediaStore.selectedMedia.mediaType === 'image'"
          :src="convertFileSrc(mediaStore.selectedMedia.path)"
          class="w-full h-full object-contain"
        />
        <video
          v-else
          :src="convertFileSrc(mediaStore.selectedMedia.path)"
          controls
          class="w-full h-full object-contain"
        />
      </div>

      <div class="bg-[#14141b] border border-[#22222c] rounded-2xl p-3 flex flex-col gap-2 text-xs">
        <div class="flex items-center justify-between pb-1.5 border-b border-[#1f1f28]">
          <span
            class="font-medium text-neutral-200 truncate flex-1"
            :title="mediaStore.selectedMedia.name"
          >
            {{ mediaStore.selectedMedia.name }}
          </span>
          <span
            class="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded bg-[#1f1f28] text-neutral-400"
          >
            {{
              mediaStore.selectedMedia.mimeType.split('/')[1] || mediaStore.selectedMedia.mediaType
            }}
          </span>
        </div>

        <div class="grid grid-cols-2 gap-2 text-[11px] text-neutral-400 font-mono">
          <div>
            Size:
            <span class="text-neutral-200">{{ formatBytes(mediaStore.selectedMedia.size) }}</span>
          </div>
          <div>
            Modified:
            <span class="text-neutral-200">{{
              formatDate(mediaStore.selectedMedia.lastModified)
            }}</span>
          </div>
        </div>
      </div>

      <div v-if="mediaStore.selectedMediaArtifact" class="flex flex-col gap-3.5">
        <!-- Fidelity Score -->
        <div
          v-if="fidelityDisplay && fidelityScore !== null"
          class="bg-[#14141b] border border-[#22222c] rounded-2xl p-3.5 flex flex-col gap-2 shadow-sm"
        >
          <div class="flex items-center justify-between">
            <span class="text-xs font-semibold uppercase tracking-wider text-neutral-400">
              Observation Fidelity
            </span>
            <div class="flex items-baseline gap-1">
              <span class="font-mono font-bold text-sm" :class="fidelityDisplay.text">
                {{ fidelityScore.toFixed(1) }}
              </span>
              <span class="text-[11px] text-neutral-500 font-mono">/ 10</span>
            </div>
          </div>

          <div class="w-full h-2 bg-[#20202a] rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-500"
              :class="fidelityDisplay.color"
              :style="{ width: `${fidelityScore * 10}%` }"
            />
          </div>

          <div class="flex items-center justify-between text-[11px]">
            <span :class="fidelityDisplay.text" class="font-medium">{{
              fidelityDisplay.label
            }}</span>
            <span class="text-[10px] font-mono text-neutral-400 uppercase">
              {{ assessment?.viewAngle || 'Angle N/A' }}
            </span>
          </div>
        </div>

        <!-- Visual Summary -->
        <div class="bg-[#14141b] p-3 rounded-2xl border border-[#22222c] flex flex-col gap-1">
          <span class="text-[10px] font-semibold text-neutral-500 uppercase tracking-wider"
            >Visual Summary</span
          >
          <p class="text-xs text-neutral-300 leading-relaxed">
            {{ assessment?.salientVisualSummary || 'No visual summary available.' }}
          </p>
        </div>

        <!-- Observation Quality Metrics -->
        <div
          v-if="assessment"
          class="bg-[#14141b] border border-[#22222c] rounded-2xl p-3.5 flex flex-col gap-2.5 text-xs"
        >
          <span class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider">
            Quality &amp; Capture Metrics
          </span>
          <div class="grid grid-cols-2 gap-2 text-[11px]">
            <div class="flex flex-col bg-[#101016] p-2 rounded-xl border border-[#1e1e28]">
              <span class="text-neutral-500 text-[10px]">Resolution</span>
              <span class="text-neutral-200 font-medium capitalize">{{
                assessment.observationQuality.resolutionClass
              }}</span>
            </div>
            <div class="flex flex-col bg-[#101016] p-2 rounded-xl border border-[#1e1e28]">
              <span class="text-neutral-500 text-[10px]">Illumination</span>
              <span class="text-neutral-200 font-medium capitalize">{{
                assessment.observationQuality.illumination.replace('_', ' ')
              }}</span>
            </div>
            <div class="flex flex-col bg-[#101016] p-2 rounded-xl border border-[#1e1e28]">
              <span class="text-neutral-500 text-[10px]">Occlusion</span>
              <span class="text-neutral-200 font-medium capitalize">{{
                assessment.observationQuality.occlusionLevel.replace('_', ' ')
              }}</span>
            </div>
            <div class="flex flex-col bg-[#101016] p-2 rounded-xl border border-[#1e1e28]">
              <span class="text-neutral-500 text-[10px]">Motion Blur</span>
              <span class="text-neutral-200 font-medium capitalize">{{
                assessment.observationQuality.motionBlur
              }}</span>
            </div>
          </div>
        </div>

        <!-- Distinctive Marks -->
        <div
          v-if="assessment && assessment.distinctiveFeatures.length > 0"
          class="bg-[#14141b] border border-[#22222c] rounded-2xl p-3.5 flex flex-col gap-2 text-xs"
        >
          <span class="text-[11px] font-semibold text-emerald-400 uppercase tracking-wider">
            Distinctive Anchor Markers
          </span>
          <div class="flex flex-wrap gap-1.5">
            <span
              v-for="feat in assessment.distinctiveFeatures"
              :key="feat"
              class="px-2 py-1 bg-emerald-950/60 border border-emerald-800/80 rounded-lg text-emerald-300 text-[10px]"
            >
              {{ feat }}
            </span>
          </div>
        </div>

        <!-- Raw JSON Extraction Profile -->
        <details class="bg-[#14141b] border border-[#22222c] rounded-2xl overflow-hidden text-xs">
          <summary
            class="px-3 py-2 cursor-pointer font-medium text-neutral-400 hover:text-neutral-200 select-none flex items-center justify-between"
          >
            <span>Raw Assessment JSON</span>
            <span class="text-[10px] text-neutral-500">▼</span>
          </summary>
          <pre
            class="p-3 text-[10px] font-mono text-neutral-400 overflow-x-auto border-t border-[#1f1f28] leading-relaxed max-h-48"
          >
        {{ JSON.stringify(mediaStore.selectedMediaArtifact, null, 2) }}</pre>
        </details>
      </div>

      <div
        v-else
        class="p-4 bg-[#14141b] border border-[#22222c] rounded-2xl flex flex-col items-center justify-center gap-1.5 text-center text-xs text-neutral-500"
      >
        <span class="text-neutral-400 font-medium">No assessment extracted yet.</span>
        <span class="text-[11px] text-neutral-600"
          >Run auto-grouping to evaluate visual fidelity.</span
        >
      </div>
    </div>

    <!-- 3. EMPTY STATE -->
    <div
      v-else
      class="flex-1 flex flex-col items-center justify-center p-6 text-center text-neutral-500 gap-3"
    >
      <svg
        class="w-10 h-10 opacity-30"
        fill="none"
        stroke="currentColor"
        stroke-width="1.5"
        viewBox="0 0 24 24"
      >
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"
        />
      </svg>
      <div class="flex flex-col gap-1 max-w-xs">
        <span class="text-xs text-neutral-300 font-medium">Select an asset from the gallery</span>
        <span class="text-[11px] text-neutral-500 leading-relaxed">
          Inspect visual feature profiles, observation quality metrics, and cluster membership.
        </span>
      </div>

      <div
        class="mt-4 w-full bg-[#14141b] border border-[#20202a] rounded-2xl p-3 flex flex-col gap-2 text-left text-xs font-mono"
      >
        <div class="flex justify-between">
          <span class="text-neutral-400">Total Scanned:</span>
          <span class="text-neutral-200 font-bold">{{ mediaStore.mediaItems.length }}</span>
        </div>
        <div class="flex justify-between">
          <span class="text-neutral-400">Profiled in Cache:</span>
          <span class="text-emerald-400 font-bold">{{ mediaStore.cachedArtifactsCount }}</span>
        </div>
      </div>
    </div>
  </aside>
</template>