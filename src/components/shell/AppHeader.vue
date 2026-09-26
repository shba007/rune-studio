<script setup lang="ts">
import { useMediaStore } from '@/stores/mediaStore'
import { useMediaComparator } from '@/composables/useMediaComparator'

const mediaStore = useMediaStore()
const { checkVisionCapability, checkServerHealth } = useMediaComparator()

async function handleModelChange(e: Event) {
  const target = e.target as HTMLSelectElement
  if (!target.value) return
  mediaStore.setModel(target.value)
  await checkVisionCapability(target.value)
}

async function handleRefreshModels() {
  await checkServerHealth()
  if (mediaStore.detectedModelName) {
    await checkVisionCapability()
  }
}
</script>

<template>
  <header
    class="h-12 bg-[#121216] border-b border-[#222228] px-5 flex items-center justify-between select-none z-10 shrink-0"
  >
    <div class="flex items-center gap-3">
      <div
        class="flex items-center gap-2 bg-[#18181e] border border-[#2a2a32] px-3 py-1 rounded-xl text-xs shadow-sm"
      >
        <span
          class="w-2 h-2 rounded-full shrink-0"
          :class="
            mediaStore.isVisionCapable === true
              ? 'bg-emerald-400 shadow-[0_0_8px_rgba(52,211,153,0.6)]'
              : mediaStore.isVisionCapable === false
                ? 'bg-red-400'
                : 'bg-amber-400 animate-pulse'
          "
        />

        <select
          v-if="mediaStore.availableModels.length > 0"
          :value="mediaStore.detectedModelName"
          @change="handleModelChange"
          class="bg-transparent text-neutral-100 font-mono text-xs font-semibold outline-none cursor-pointer pr-1"
        >
          <option v-if="!mediaStore.detectedModelName" value="" disabled>Select model...</option>
          <option
            v-for="model in mediaStore.availableModels"
            :key="model"
            :value="model"
            class="bg-[#18181e] text-neutral-200"
          >
            {{ model }}
          </option>
        </select>
        <span v-else class="text-xs text-neutral-400 italic">
          {{ mediaStore.isFetchingModels ? 'Querying /health...' : 'No models detected' }}
        </span>

        <button
          type="button"
          @click="handleRefreshModels"
          :disabled="mediaStore.isFetchingModels"
          title="Query /health and models endpoint"
          class="p-0.5 hover:text-neutral-200 text-neutral-500 transition-colors"
        >
          <svg
            class="w-3.5 h-3.5"
            :class="{ 'animate-spin': mediaStore.isFetchingModels }"
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

        <span class="text-[10px] text-neutral-500 font-mono border-l border-[#2e2e38] pl-2">
          {{
            mediaStore.serverHealth?.version ? `v${mediaStore.serverHealth.version}` : 'Rune Infer'
          }}
        </span>
      </div>

      <button
        type="button"
        @click="mediaStore.isThinkingEnabled = !mediaStore.isThinkingEnabled"
        class="flex items-center gap-1.5 px-2.5 py-1 rounded-xl text-xs border transition-colors font-medium"
        :class="
          mediaStore.isThinkingEnabled
            ? 'bg-[#18181e] text-emerald-400 border-emerald-900/60'
            : 'bg-[#141418] text-neutral-500 border-[#222228]'
        "
      >
        <span>💡</span>
        <span>Thinking • {{ mediaStore.isThinkingEnabled ? 'Deep (0.6)' : 'Strict (0.2)' }}</span>
      </button>
    </div>

    <div class="flex items-center gap-3">
      <div
        class="flex items-center gap-2.5 bg-[#18181e] border border-[#2a2a32] px-3 py-1 rounded-xl text-xs"
      >
        <span class="text-[11px] text-neutral-400">Match Threshold:</span>
        <input
          v-model.number="mediaStore.confidenceThreshold"
          type="range"
          min="0.4"
          max="1.0"
          step="0.05"
          class="w-20 accent-emerald-500 cursor-pointer h-1 bg-[#262630] rounded-lg"
        />
        <span class="font-mono font-bold text-emerald-400 text-xs w-8 text-right">
          {{ Math.round(mediaStore.confidenceThreshold * 100) }}%
        </span>
      </div>
    </div>
  </header>
</template>