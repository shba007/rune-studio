<script setup lang="ts">
import { onMounted } from 'vue'
import { useMediaStore, COMPARISON_INTENTS } from '@/stores/mediaStore'
import { useMediaScanner } from '@/composables/useMediaScanner'
import { useMediaComparator } from '@/composables/useMediaComparator'
import MediaGallery from '@/components/comparator/MediaGallery.vue'
import ComparisonDock from '@/components/comparator/ComparisonDock.vue'
import StagedActionsModal from '@/components/comparator/StagedActionsModal.vue'

const mediaStore = useMediaStore()
const { isScanning, scanError, pickDirectory, scanDirectory } = useMediaScanner()
const { checkVisionCapability } = useMediaComparator()

onMounted(async () => {
  await checkVisionCapability()
})

async function handleBrowse() {
  const chosenPath = await pickDirectory()
  if (chosenPath) {
    mediaStore.setDirectory(chosenPath)
    await handleScan()
  }
}

async function handleScan() {
  if (!mediaStore.directoryPath) return
  const items = await scanDirectory(mediaStore.directoryPath, mediaStore.isRecursive)
  mediaStore.setMediaItems(items)
}
</script>

<template>
  <div class="w-full h-full flex flex-col bg-[#0c0c0e] text-neutral-200 overflow-hidden">
    <!-- Sub-header: Directory Scan & Context Intent Toolbar -->
    <div
      class="border-b border-[#1f1f26] bg-[#121217] px-5 py-2.5 flex flex-col gap-2.5 z-10 shrink-0"
    >
      <!-- Top Row: Directory Input -->
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-2 flex-1 max-w-2xl">
          <div class="relative flex-1">
            <input
              v-model="mediaStore.directoryPath"
              @keyup.enter="handleScan"
              type="text"
              placeholder="Select folder or enter absolute path..."
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
          <label
            class="flex items-center gap-1.5 text-xs text-neutral-400 cursor-pointer select-none"
          >
            <input
              v-model="mediaStore.isRecursive"
              type="checkbox"
              class="rounded border-[#2e2e38] bg-[#18181f] text-emerald-600 focus:ring-0"
            />
            Recursive
          </label>
          <button
            type="button"
            :disabled="isScanning || !mediaStore.directoryPath"
            @click="handleScan"
            class="px-3.5 py-1.5 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 text-white text-xs font-semibold rounded-xl shadow-md transition-all flex items-center gap-1.5"
          >
            <svg v-if="isScanning" class="w-3 h-3 animate-spin" fill="none" viewBox="0 0 24 24">
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
            <span>{{ isScanning ? 'Scanning...' : 'Scan Directory' }}</span>
          </button>
        </div>
      </div>

      <!-- Bottom Row: Intent Selector Pills (Unsloth Style) -->
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

        <!-- Custom Prompt Input (if selected) -->
        <div v-if="mediaStore.selectedIntentKey === 'custom'" class="w-80">
          <input
            v-model="mediaStore.customPromptText"
            type="text"
            placeholder="Type custom prompt rule..."
            class="w-full bg-[#18181f] border border-[#272732] rounded-xl px-2.5 py-1 text-xs text-neutral-200 outline-none focus:border-emerald-600"
          />
        </div>
      </div>

      <!-- Active Intent Subtitle -->
      <p class="text-[11px] text-neutral-400 flex items-center gap-1.5 px-0.5">
        <span class="text-neutral-500">Evaluation mode:</span>
        <span class="text-neutral-300 font-medium">{{ mediaStore.currentIntent.description }}</span>
      </p>
    </div>

    <!-- Scan Error Alert -->
    <div
      v-if="scanError"
      class="px-5 py-2 bg-red-950/80 border-b border-red-800 text-red-200 text-xs"
    >
      {{ scanError }}
    </div>

    <!-- Main Workspace Split: Media Gallery & Comparison Dock -->
    <main class="flex-1 flex overflow-hidden">
      <MediaGallery />
      <ComparisonDock />
    </main>

    <!-- Bottom Staged Action Bar -->
    <footer
      class="h-12 border-t border-[#1f1f26] bg-[#101014] px-5 flex items-center justify-between text-xs select-none shrink-0"
    >
      <div class="flex items-center gap-2 text-neutral-400">
        <span
          class="w-2 h-2 rounded-full"
          :class="
            mediaStore.activeStagedCount > 0 ? 'bg-amber-400 animate-pulse' : 'bg-neutral-600'
          "
        />
        <span>
          Staged Operations: <strong>{{ mediaStore.activeStagedCount }} pending</strong>
        </span>
      </div>

      <button
        :disabled="mediaStore.activeStagedCount === 0"
        @click="mediaStore.isReviewModalOpen = true"
        class="px-4 py-1.5 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:bg-[#1a1a20] disabled:text-neutral-500 text-white font-medium rounded-xl shadow-md transition-all"
      >
        Review &amp; Execute ({{ mediaStore.activeStagedCount }})
      </button>
    </footer>

    <!-- Permission Review Modal Dialog -->
    <StagedActionsModal />
  </div>
</template>