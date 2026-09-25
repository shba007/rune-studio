<script setup lang="ts">
import { ref } from 'vue'
import { useMediaStore } from '@/stores/mediaStore'
import { useMediaScanner } from '@/composables/useMediaScanner'

const mediaStore = useMediaStore()
const { executeFileActions } = useMediaScanner()

const isExecuting = ref(false)
const executionFeedback = ref<{ success: number; errors: string[] } | null>(null)

async function handleConfirmExecution() {
  const activeActions = mediaStore.stagedActions.filter((a) => a.enabled)
  if (activeActions.length === 0) return

  isExecuting.value = true
  executionFeedback.value = null

  try {
    const payload = activeActions.map((a) => ({
      actionType: 'MOVE' as const,
      sourcePath: a.sourceItem.path,
      targetDirectory: a.targetDirectory || `${mediaStore.directoryPath}/_staged`,
      targetFileName: a.sourceItem.name,
    }))

    const res = await executeFileActions(payload)
    executionFeedback.value = {
      success: res.successCount,
      errors: res.errors,
    }

    if (res.errors.length === 0) {
      mediaStore.clearStagedActions()
    }
  } catch (err: unknown) {
    const msg = err instanceof Error ? err.message : String(err)
    executionFeedback.value = {
      success: 0,
      errors: [msg],
    }
  } finally {
    isExecuting.value = false
  }
}
</script>

<template>
  <div
    v-if="mediaStore.isReviewModalOpen"
    class="fixed inset-0 z-50 bg-black/80 backdrop-blur-sm flex items-center justify-center p-4"
  >
    <div
      class="w-full max-w-2xl bg-neutral-900 border border-neutral-800 rounded-3xl shadow-2xl flex flex-col overflow-hidden max-h-[85vh]"
    >
      <div class="px-6 py-4 border-b border-neutral-800 flex items-center justify-between">
        <div>
          <h3 class="text-sm font-semibold text-neutral-100">Review Staged File Operations</h3>
          <p class="text-xs text-neutral-400">
            Explicit confirmation required before applying file movements.
          </p>
        </div>
        <button
          @click="mediaStore.isReviewModalOpen = false"
          class="text-neutral-400 hover:text-neutral-200 text-lg leading-none"
        >
          &times;
        </button>
      </div>

      <div
        class="mx-6 mt-4 p-3 bg-blue-950/40 border border-blue-800/80 rounded-2xl flex items-center gap-2.5 text-xs text-blue-200"
      >
        <svg
          class="w-4 h-4 text-blue-400 flex-shrink-0"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z"
          />
        </svg>
        <span
          >No files are destroyed. Duplicates are safely segregated into quarantine for
          review.</span
        >
      </div>

      <div
        v-if="executionFeedback"
        class="mx-6 mt-3 p-3 rounded-2xl text-xs"
        :class="
          executionFeedback.errors.length === 0
            ? 'bg-emerald-950/60 border border-emerald-800 text-emerald-200'
            : 'bg-red-950/60 border border-red-800 text-red-200'
        "
      >
        <p class="font-medium">Successfully processed: {{ executionFeedback.success }} files.</p>
        <p
          v-for="err in executionFeedback.errors"
          :key="err"
          class="text-red-300 mt-1 font-mono text-[11px]"
        >
          {{ err }}
        </p>
      </div>

      <div class="flex-1 overflow-y-auto px-6 py-3 flex flex-col gap-2">
        <div
          v-for="action in mediaStore.stagedActions"
          :key="action.id"
          class="p-3 bg-neutral-950/60 border border-neutral-800 rounded-2xl flex items-center justify-between gap-3 text-xs"
        >
          <input
            type="checkbox"
            :checked="action.enabled"
            @change="mediaStore.toggleStagedAction(action.id)"
            class="rounded border-neutral-700 bg-neutral-800 text-emerald-600 focus:ring-0 cursor-pointer"
          />

          <div class="flex-1 flex flex-col gap-0.5 overflow-hidden">
            <div class="flex items-center gap-2">
              <span class="font-medium text-neutral-200 truncate">{{
                action.sourceItem.name
              }}</span>
              <span
                class="px-1.5 py-0.5 text-[10px] rounded font-mono uppercase"
                :class="
                  action.type === 'QUARANTINE_DUPLICATE'
                    ? 'bg-amber-950 text-amber-300 border border-amber-800'
                    : 'bg-blue-950 text-blue-300 border border-blue-800'
                "
              >
                {{ action.type === 'QUARANTINE_DUPLICATE' ? 'Quarantine' : 'Group' }}
              </span>
            </div>
            <span class="text-[11px] text-neutral-400 font-mono truncate">
              Destination: {{ action.targetDirectory }}
            </span>
          </div>

          <button
            @click="mediaStore.removeStagedAction(action.id)"
            class="text-neutral-500 hover:text-neutral-300 p-1"
          >
            &times;
          </button>
        </div>
      </div>

      <div
        class="px-6 py-4 border-t border-neutral-800 flex items-center justify-between bg-neutral-950/40"
      >
        <button
          type="button"
          @click="mediaStore.clearStagedActions()"
          class="text-xs text-neutral-500 hover:text-neutral-300"
        >
          Clear All
        </button>

        <div class="flex items-center gap-3">
          <button
            type="button"
            @click="mediaStore.isReviewModalOpen = false"
            class="px-4 py-2 text-xs font-medium text-neutral-400 hover:text-neutral-200 transition-colors"
          >
            Cancel
          </button>

          <button
            type="button"
            :disabled="mediaStore.activeStagedCount === 0 || isExecuting"
            @click="handleConfirmExecution"
            class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 text-white text-xs font-semibold rounded-xl shadow-lg transition-all flex items-center gap-1.5"
          >
            <svg
              v-if="isExecuting"
              class="w-3.5 h-3.5 animate-spin"
              fill="none"
              viewBox="0 0 24 24"
            >
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
            <span>Confirm &amp; Apply Changes ({{ mediaStore.activeStagedCount }})</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>