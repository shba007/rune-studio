<script setup lang="ts">
import { onMounted } from 'vue'
import { convertFileSrc } from '@tauri-apps/api/core'
import { useMediaStore, COMPARISON_INTENTS } from '@/stores/mediaStore'
import { useMediaScanner } from '@/composables/useMediaScanner'
import { useMediaComparator } from '@/composables/useMediaComparator'
import { useMediaProfiler } from '@/composables/useMediaProfiler'
import { useAutoPipeline } from '@/composables/useAutoPipeline'
import MediaGallery from '@/components/comparator/MediaGallery.vue'
import MediaInspectorDock from '@/components/comparator/MediaInspectorDock.vue'
import StagedActionsModal from '@/components/comparator/StagedActionsModal.vue'
import IdentityGroupingModal from '@/components/comparator/IdentityGroupingModal.vue'

const mediaStore = useMediaStore()
const { isScanning, scanError, pickDirectory, scanDirectory, generateThumbnails } =
  useMediaScanner()
const { checkVisionCapability } = useMediaComparator()
const { getAssessment, purgeDirectoryCache } = useMediaProfiler()
const { startAutoPipeline, cancelPipeline } = useAutoPipeline()

async function handlePurgeDirectoryCache() {
  if (!mediaStore.directoryPath) return
  if (confirm('Clear all cached assessments and arbitration logs for this folder?')) {
    await purgeDirectoryCache()
  }
}

onMounted(async () => {
  await checkVisionCapability()
})

async function preloadAssessments(items: typeof mediaStore.mediaItems) {
  for (const item of items) {
    await getAssessment(item)
  }
}

async function handleScan(path = mediaStore.directoryPath) {
  if (!path.trim()) return
  const items = await scanDirectory(path, mediaStore.isRecursive)
  mediaStore.setMediaItems(items)

  // Background thumbnail batch generation for un-cached items
  const missing = items.filter((it) => !it.thumbnailPath)
  if (missing.length > 0) {
    void generateThumbnails(missing, path).then((res) => {
      const updates = res.map((r) => ({
        id: r.id,
        thumbnailUrl: convertFileSrc(r.thumbnailPath),
        thumbnailPath: r.thumbnailPath,
      }))
      mediaStore.updateThumbnails(updates)
    })
  }

  void preloadAssessments(items)
}

async function handleBrowse() {
  const chosenPath = await pickDirectory()
  if (chosenPath) {
    mediaStore.setDirectory(chosenPath)
    await handleScan(chosenPath)
  }
}
</script>

<template>
  <div class="w-full h-full flex flex-col bg-[#0c0c0e] text-neutral-200 overflow-hidden">
    <!-- Top Configuration & Auto-Scan Bar -->
    <div
      class="border-b border-[#1f1f26] bg-[#121217] px-5 py-2.5 flex flex-col gap-2.5 z-10 shrink-0 select-none"
    >
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-2 flex-1 max-w-3xl">
          <div class="relative flex-1">
            <input
              v-model="mediaStore.directoryPath"
              @keyup.enter="handleScan()"
              type="text"
              placeholder="Select folder or enter path..."
              class="w-full bg-[#18181f] border border-[#272732] rounded-xl px-3.5 py-1.5 text-xs font-mono text-neutral-200 outline-none focus:border-emerald-600 transition-colors"
            />
          </div>

          <button
            type="button"
            @click="handleBrowse"
            class="px-3 py-1.5 bg-[#1e1e26] hover:bg-[#262632] text-xs font-medium text-neutral-200 rounded-xl border border-[#2d2d3a] transition-all flex items-center gap-1.5 shadow-sm"
          >
            <svg
              class="w-3.5 h-3.5 text-neutral-400"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
              />
            </svg>
            Browse
          </button>

          <button
            type="button"
            @click="handleScan()"
            :disabled="isScanning || !mediaStore.directoryPath"
            title="Refresh & rescan folder"
            class="p-1.5 bg-[#1e1e26] hover:bg-[#262632] disabled:opacity-40 text-neutral-300 rounded-xl border border-[#2d2d3a] transition-all"
          >
            <svg
              class="w-4 h-4"
              :class="{ 'animate-spin': isScanning }"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
              />
            </svg>
          </button>

          <button
            type="button"
            @click="handlePurgeDirectoryCache"
            :disabled="isScanning || !mediaStore.directoryPath"
            title="Purge .rune_cache for this entire folder"
            class="p-1.5 bg-[#1e1e26] hover:bg-red-950/40 hover:text-red-400 text-neutral-400 rounded-xl border border-[#2d2d3a] hover:border-red-900/60 transition-all"
          >
            <svg
              class="w-4 h-4"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="m14.74 9-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 0 1-2.244 2.077H8.084a2.25 2.25 0 0 1-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 0 0-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 0 1 3.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 0 0-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 0 0-7.5 0"
              />
            </svg>
          </button>

          <label
            class="flex items-center gap-1.5 text-xs text-neutral-400 cursor-pointer select-none"
          >
            <input
              v-model="mediaStore.isRecursive"
              type="checkbox"
              @change="handleScan()"
              class="rounded border-[#2e2e38] bg-[#18181f] text-emerald-600 focus:ring-0"
            />
            Recursive
          </label>

          <button
            v-if="mediaStore.pipelineState === 'idle' || mediaStore.pipelineState === 'completed'"
            type="button"
            :disabled="mediaStore.mediaItems.length === 0 || isScanning"
            @click="startAutoPipeline"
            class="px-4 py-1.5 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 text-white text-xs font-semibold rounded-xl shadow-md transition-all flex items-center gap-1.5"
          >
            <span>
              {{
                mediaStore.selectedIntentKey === 'duplicates'
                  ? '⚡ Find Duplicates'
                  : '⚡ Auto-Group All'
              }}
            </span>
          </button>

          <button
            v-else
            type="button"
            @click="cancelPipeline"
            class="px-4 py-1.5 bg-red-900/80 hover:bg-red-800 border border-red-700 text-white text-xs font-semibold rounded-xl shadow-md transition-all flex items-center gap-1.5"
          >
            <span>Cancel</span>
          </button>
        </div>
      </div>

      <div class="flex items-center justify-between gap-3">
        <div class="flex items-center gap-1.5 overflow-x-auto py-0.5">
          <button
            v-for="(intent, key) in COMPARISON_INTENTS"
            :key="key"
            @click="mediaStore.selectedIntentKey = String(key)"
            type="button"
            class="px-3 py-1 rounded-xl text-xs font-medium border transition-all flex items-center gap-1.5 shrink-0"
            :class="
              mediaStore.selectedIntentKey === key
                ? 'bg-[#1e1e28] text-emerald-400 border-emerald-800 shadow-sm'
                : 'bg-[#15151b] text-neutral-400 border-[#242430] hover:text-neutral-200 hover:bg-[#1a1a22]'
            "
          >
            <span>{{ intent.badge }}</span>
          </button>
        </div>
      </div>

      <p class="text-[11px] text-neutral-400 flex items-center gap-1.5 px-0.5">
        <span class="text-neutral-500">Mode:</span>
        <span class="text-neutral-300 font-medium">{{ mediaStore.currentIntent.description }}</span>
      </p>
    </div>

    <div
      v-if="scanError"
      class="px-5 py-2 bg-red-950/80 border-b border-red-800 text-red-200 text-xs"
    >
      {{ scanError }}
    </div>

    <!-- Main Workspace -->
    <main class="flex-1 flex overflow-hidden">
      <MediaGallery />
      <MediaInspectorDock />
    </main>

    <!-- Footer with Processing Progress -->
    <footer
      class="relative h-10 border-t border-[#1f1f26] bg-[#101014] px-5 flex items-center justify-between text-xs select-none shrink-0 font-mono text-neutral-500 overflow-hidden"
    >
      <div
        v-if="mediaStore.pipelineState === 'profiling' || mediaStore.pipelineState === 'clustering'"
        class="absolute top-0 left-0 right-0 h-0.5 bg-[#20202c]"
      >
        <div
          class="h-full bg-emerald-500 transition-all duration-300 shadow-[0_0_8px_rgba(16,185,129,0.5)]"
          :style="{
            width: `${(mediaStore.pipelineProgress.current / Math.max(mediaStore.pipelineProgress.total, 1)) * 100}%`,
          }"
        />
      </div>

      <div class="flex items-center gap-4 z-10">
        <span
          >Files: <strong class="text-neutral-300">{{ mediaStore.mediaItems.length }}</strong></span
        >
        <span
          >Profiled:
          <strong class="text-emerald-400">{{ mediaStore.cachedArtifactsCount }}</strong></span
        >
      </div>

      <div
        v-if="mediaStore.pipelineState === 'profiling' || mediaStore.pipelineState === 'clustering'"
        class="flex items-center gap-3 flex-1 max-w-lg mx-6 z-10"
      >
        <span
          class="text-[11px] text-neutral-400 truncate max-w-[160px]"
          :title="mediaStore.pipelineProgress.currentItem?.name"
        >
          {{
            mediaStore.pipelineProgress.currentItem?.name ||
            mediaStore.pipelineProgress.currentStage
          }}
        </span>
        <div class="flex-1 h-1.5 bg-[#20202c] rounded-full overflow-hidden">
          <div
            class="h-full bg-emerald-500 transition-all duration-300 rounded-full"
            :style="{
              width: `${(mediaStore.pipelineProgress.current / Math.max(mediaStore.pipelineProgress.total, 1)) * 100}%`,
            }"
          />
        </div>
        <span class="text-[11px] text-emerald-400 font-semibold shrink-0">
          {{ mediaStore.pipelineProgress.current }} / {{ mediaStore.pipelineProgress.total }}
        </span>
      </div>

      <div class="flex items-center gap-2.5 z-10">
        <span
          class="w-2 h-2 rounded-full"
          :class="
            mediaStore.pipelineState === 'profiling' || mediaStore.pipelineState === 'clustering'
              ? 'bg-emerald-400 animate-pulse'
              : 'bg-neutral-600'
          "
        />
        <span class="capitalize text-neutral-400">Status: {{ mediaStore.pipelineState }}</span>
      </div>
    </footer>

    <!-- Interactive Modals -->
    <IdentityGroupingModal />
    <StagedActionsModal />
  </div>
</template>