<script setup lang="ts">
import { useMediaStore } from '@/stores/mediaStore'
import { useMediaComparator } from '@/composables/useMediaComparator'

const mediaStore = useMediaStore()
const {
  isComparing,
  error: comparisonError,
  compareMediaPair,
  checkVisionCapability,
} = useMediaComparator()

async function handleModelChange(e: Event) {
  const target = e.target as HTMLSelectElement
  if (!target.value) return
  mediaStore.setModel(target.value)
  await checkVisionCapability(target.value)
}

async function handleRunComparison() {
  if (!mediaStore.selectedItemA || !mediaStore.selectedItemB) return

  try {
    const verdict = await compareMediaPair(
      mediaStore.selectedItemA,
      mediaStore.selectedItemB,
      mediaStore.effectivePrompt,
    )
    mediaStore.setComparisonVerdict(verdict)
  } catch (err) {
    console.error('Comparison error:', err)
  }
}

function stageKeepSameDirectory() {
  if (!mediaStore.selectedItemA || !mediaStore.selectedItemB || !mediaStore.lastVerdict) return

  const targetCategory =
    mediaStore.lastVerdict.similarityType?.replace(/[^a-zA-Z0-9_-]/g, '_') || 'matched_group'
  const targetDir = `${mediaStore.directoryPath}/_organized_${targetCategory}`

  mediaStore.stageAction({
    type: 'MOVE_SAME_DIR',
    sourceItem: mediaStore.selectedItemB,
    matchedWith: mediaStore.selectedItemA,
    targetDirectory: targetDir,
    confidence: mediaStore.lastVerdict.confidence,
    reason: `Grouped into category: ${targetCategory}`,
  })
}

function stageDeduplication() {
  if (!mediaStore.selectedItemA || !mediaStore.selectedItemB || !mediaStore.lastVerdict) return

  const isAPrimary = mediaStore.lastVerdict.recommendedPrimary !== 'item_b'
  const duplicateItem = isAPrimary ? mediaStore.selectedItemB : mediaStore.selectedItemA
  const primaryItem = isAPrimary ? mediaStore.selectedItemA : mediaStore.selectedItemB

  const targetDir = `${mediaStore.directoryPath}/_quarantine_shredder`

  mediaStore.stageAction({
    type: 'QUARANTINE_DUPLICATE',
    sourceItem: duplicateItem,
    matchedWith: primaryItem,
    targetDirectory: targetDir,
    confidence: mediaStore.lastVerdict.confidence,
    reason: `Duplicate of ${primaryItem.name} (${Math.round(mediaStore.lastVerdict.confidence * 100)}% match)`,
  })
}
</script>

<template>
  <aside
    class="w-[450px] h-full border-l border-[#1f1f26] bg-[#101015] flex flex-col overflow-y-auto shrink-0"
  >
    <div class="p-3.5 border-b border-[#1f1f26] flex flex-col gap-2 bg-[#121217]">
      <div class="flex items-center justify-between">
        <h2 class="text-xs font-semibold uppercase tracking-wider text-neutral-400">
          Pair Inspection
        </h2>
        <button
          v-if="mediaStore.selectedItemA || mediaStore.selectedItemB"
          @click="mediaStore.clearSelection()"
          class="text-xs text-neutral-500 hover:text-neutral-300 transition-colors"
        >
          Clear Pair
        </button>
      </div>

      <div
        class="flex items-center justify-between gap-2 bg-[#17171e] border border-[#272732] px-2.5 py-1.5 rounded-xl text-xs"
      >
        <span class="text-neutral-400 text-[11px]">Model:</span>
        <select
          v-if="mediaStore.availableModels.length > 0"
          :value="mediaStore.detectedModelName"
          @change="handleModelChange"
          class="bg-transparent text-emerald-400 font-mono text-xs font-semibold outline-none cursor-pointer flex-1"
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
        <span v-else class="text-xs text-neutral-500 italic flex-1">
          {{ mediaStore.isFetchingModels ? 'Loading...' : 'No models' }}
        </span>

        <span
          class="px-1.5 py-0.5 text-[9px] font-mono rounded border uppercase tracking-wider"
          :class="
            mediaStore.isVisionCapable === true
              ? 'bg-emerald-950/60 text-emerald-400 border-emerald-800'
              : mediaStore.isVisionCapable === false
                ? 'bg-red-950/60 text-red-400 border-red-800'
                : 'bg-[#181820] text-neutral-400 border-[#262630]'
          "
        >
          {{
            mediaStore.isVisionCapable === true
              ? 'Vision Ready'
              : mediaStore.isVisionCapable === false
                ? 'No Vision'
                : 'Verifying'
          }}
        </span>
      </div>
    </div>

    <div
      v-if="mediaStore.isVisionCapable === false"
      class="mx-4 mt-4 p-3.5 bg-red-950/40 border border-red-800/80 rounded-2xl flex flex-col gap-2 text-xs text-red-200"
    >
      <div class="flex items-center gap-2 font-semibold">
        <svg
          class="w-4 h-4 text-red-400 shrink-0"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z"
          />
        </svg>
        <span>Text-Only Model Active</span>
      </div>
      <p class="text-[11px] text-red-300/90 leading-relaxed">
        The active model (<strong>{{ mediaStore.detectedModelName }}</strong
        >) does not process visual inputs. Select a vision model from the dropdown to run
        comparisons.
      </p>
      <button
        type="button"
        @click="() => checkVisionCapability()"
        class="self-start px-2.5 py-1 bg-red-900/60 hover:bg-red-800 border border-red-700 rounded-lg text-[11px] font-medium text-white transition-colors"
      >
        Re-verify Model
      </button>
    </div>

    <div class="p-4 flex flex-col gap-4">
      <div class="grid grid-cols-2 gap-3">
        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs font-medium">
            <span class="text-blue-400 flex items-center gap-1.5">
              <span
                class="w-4 h-4 rounded-full bg-blue-600 text-white text-[10px] font-bold flex items-center justify-center"
                >A</span
              >
              Item A
            </span>
            <button
              v-if="mediaStore.selectedItemA"
              @click="mediaStore.selectedItemA = null"
              class="text-neutral-500 hover:text-neutral-300"
            >
              &times;
            </button>
          </div>
          <div
            class="aspect-square bg-[#15151c] border border-[#242430] rounded-2xl overflow-hidden flex items-center justify-center relative"
          >
            <template v-if="mediaStore.selectedItemA">
              <img
                v-if="mediaStore.selectedItemA.mediaType === 'image'"
                :src="mediaStore.selectedItemA.thumbnailUrl"
                class="w-full h-full object-cover"
              />
              <video
                v-else
                :src="mediaStore.selectedItemA.thumbnailUrl"
                muted
                controls
                class="w-full h-full object-cover"
              />
            </template>
            <span v-else class="text-xs text-neutral-600 text-center px-2"
              >Click item in gallery</span
            >
          </div>
          <span
            v-if="mediaStore.selectedItemA"
            class="text-[11px] text-neutral-300 truncate"
            :title="mediaStore.selectedItemA.name"
          >
            {{ mediaStore.selectedItemA.name }}
          </span>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs font-medium">
            <span class="text-emerald-400 flex items-center gap-1.5">
              <span
                class="w-4 h-4 rounded-full bg-emerald-600 text-white text-[10px] font-bold flex items-center justify-center"
                >B</span
              >
              Item B
            </span>
            <button
              v-if="mediaStore.selectedItemB"
              @click="mediaStore.selectedItemB = null"
              class="text-neutral-500 hover:text-neutral-300"
            >
              &times;
            </button>
          </div>
          <div
            class="aspect-square bg-[#15151c] border border-[#242430] rounded-2xl overflow-hidden flex items-center justify-center relative"
          >
            <template v-if="mediaStore.selectedItemB">
              <img
                v-if="mediaStore.selectedItemB.mediaType === 'image'"
                :src="mediaStore.selectedItemB.thumbnailUrl"
                class="w-full h-full object-cover"
              />
              <video
                v-else
                :src="mediaStore.selectedItemB.thumbnailUrl"
                muted
                controls
                class="w-full h-full object-cover"
              />
            </template>
            <span v-else class="text-xs text-neutral-600 text-center px-2">Click second item</span>
          </div>
          <span
            v-if="mediaStore.selectedItemB"
            class="text-[11px] text-neutral-300 truncate"
            :title="mediaStore.selectedItemB.name"
          >
            {{ mediaStore.selectedItemB.name }}
          </span>
        </div>
      </div>

      <button
        type="button"
        @click="handleRunComparison"
        :disabled="!mediaStore.canCompare || isComparing"
        class="w-full py-2.5 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 disabled:cursor-not-allowed text-white text-xs font-semibold rounded-2xl shadow-lg transition-all flex items-center justify-center gap-2"
      >
        <svg v-if="isComparing" class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
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
        <svg
          v-else
          class="w-4 h-4"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M7.5 21 3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5"
          />
        </svg>
        <span>
          {{
            isComparing
              ? 'Evaluating Visual Tokens...'
              : !mediaStore.detectedModelName
                ? 'Select Model to Compare'
                : mediaStore.isVisionCapable === false
                  ? 'Text-Only Model (Select Vision Model)'
                  : 'Compare'
          }}
        </span>
      </button>

      <div
        v-if="comparisonError"
        class="p-3 bg-red-950/80 border border-red-800 rounded-xl text-red-300 text-xs"
      >
        {{ comparisonError }}
      </div>

      <div
        v-if="mediaStore.lastVerdict"
        class="flex flex-col gap-3 bg-[#15151c] border border-[#242430] rounded-2xl p-4"
      >
        <div class="flex items-center justify-between">
          <span class="text-xs text-neutral-400">Visual Match:</span>
          <span
            class="px-2.5 py-1 text-xs font-bold rounded-lg uppercase tracking-wide"
            :class="
              mediaStore.lastVerdict.isMatch
                ? 'bg-emerald-950 text-emerald-300 border border-emerald-700'
                : 'bg-[#1e1e26] text-neutral-400 border-[#2e2e38]'
            "
          >
            {{ mediaStore.lastVerdict.isMatch ? 'Match Found' : 'No Match' }}
          </span>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between text-xs">
            <span class="text-neutral-400">AI Confidence:</span>
            <span
              class="font-mono font-bold"
              :class="mediaStore.isThresholdMet ? 'text-emerald-400' : 'text-neutral-400'"
            >
              {{ Math.round(mediaStore.lastVerdict.confidence * 100) }}%
              <span class="text-[10px] text-neutral-500 font-normal">
                (threshold: {{ Math.round(mediaStore.confidenceThreshold * 100) }}%)
              </span>
            </span>
          </div>
          <div class="w-full h-1.5 bg-[#20202a] rounded-full overflow-hidden">
            <div
              class="h-full transition-all duration-500 rounded-full"
              :class="mediaStore.isThresholdMet ? 'bg-emerald-500' : 'bg-neutral-600'"
              :style="{ width: `${mediaStore.lastVerdict.confidence * 100}%` }"
            />
          </div>
        </div>

        <div
          v-if="mediaStore.lastVerdict.visualSummaryA || mediaStore.lastVerdict.visualSummaryB"
          class="flex flex-col gap-1.5 bg-[#0e0e12] p-2.5 rounded-xl border border-[#22222a] text-[11px]"
        >
          <div v-if="mediaStore.lastVerdict.visualSummaryA">
            <span class="text-blue-400 font-semibold">Image A:</span>
            <span class="text-neutral-300 ml-1">{{ mediaStore.lastVerdict.visualSummaryA }}</span>
          </div>
          <div v-if="mediaStore.lastVerdict.visualSummaryB">
            <span class="text-emerald-400 font-semibold">Image B:</span>
            <span class="text-neutral-300 ml-1">{{ mediaStore.lastVerdict.visualSummaryB }}</span>
          </div>
        </div>

        <div class="flex items-center justify-between text-xs pt-1 border-t border-[#202028]">
          <span class="text-neutral-400">Classification:</span>
          <span class="text-neutral-200 font-medium capitalize">
            {{ mediaStore.lastVerdict.similarityType || 'Visual comparison' }}
          </span>
        </div>

        <p
          class="text-xs text-neutral-300 bg-[#0e0e12] p-3 rounded-xl border border-[#22222a] leading-relaxed"
        >
          {{ mediaStore.lastVerdict.explanation }}
        </p>

        <details
          v-if="mediaStore.lastVerdict.thought"
          class="group bg-[#0e0e12] border border-[#22222a] rounded-xl overflow-hidden text-xs"
        >
          <summary
            class="px-3 py-2 cursor-pointer font-medium text-neutral-400 hover:text-neutral-200 select-none flex items-center justify-between"
          >
            <span>Model Reasoning Trace</span>
            <span class="text-[10px] text-neutral-500 group-open:rotate-180 transition-transform"
              >▼</span
            >
          </summary>
          <div
            class="px-3 pb-3 text-neutral-400 whitespace-pre-wrap font-mono text-[11px] leading-relaxed max-h-48 overflow-y-auto border-t border-[#1e1e26] pt-2"
          >
            {{ mediaStore.lastVerdict.thought }}
          </div>
        </details>

        <div
          v-if="mediaStore.isThresholdMet"
          class="flex flex-col gap-2 pt-2 border-t border-[#202028]"
        >
          <span class="text-[11px] font-semibold text-neutral-400 uppercase tracking-wider">
            Staging Actions (Explicit Permission)
          </span>

          <button
            type="button"
            @click="stageKeepSameDirectory"
            class="px-3 py-2 bg-[#1b1b22] hover:bg-[#242430] active:bg-[#2c2c3a] text-neutral-200 text-xs font-medium rounded-xl border border-[#2a2a36] transition-all text-left flex items-center justify-between"
          >
            <span>Keep matched in same category folder</span>
            <span class="text-emerald-400 font-bold">+ Stage</span>
          </button>

          <button
            type="button"
            @click="stageDeduplication"
            class="px-3 py-2 bg-[#1b1b22] hover:bg-[#242430] active:bg-[#2c2c3a] text-neutral-200 text-xs font-medium rounded-xl border border-[#2a2a36] transition-all text-left flex items-center justify-between"
          >
            <span>Stage duplicate to review/shredder folder</span>
            <span class="text-amber-400 font-bold">+ Stage</span>
          </button>
        </div>
      </div>
    </div>
  </aside>
</template>