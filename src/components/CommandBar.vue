<script setup lang="ts">
import { ref } from 'vue'

defineProps<{
  isProcessing: boolean
  lastCommand: string
  errorMessage?: string
}>()

const emit = defineEmits<{
  (e: 'submit', prompt: string): void
}>()

const inputPrompt = ref('')

function handleSubmit() {
  if (!inputPrompt.value.trim()) return
  emit('submit', inputPrompt.value)
  inputPrompt.value = ''
}
</script>

<template>
  <aside class="absolute bottom-6 left-1/2 -translate-x-1/2 w-11/12 max-w-2xl flex flex-col gap-2 z-10 select-none">
    <!-- Error Notification -->
    <div v-if="errorMessage"
      class="bg-red-950/80 backdrop-blur-md text-red-200 border border-red-800 text-xs px-3 py-2 rounded-xl">
      {{ errorMessage }}
    </div>

    <!-- <div v-if="lastCommand"
      class="bg-neutral-900/90 backdrop-blur-md text-emerald-400 font-mono text-xs px-3 py-2 rounded-xl border border-neutral-800 shadow-xl break-all">
      <code>{{ lastCommand }}</code>
    </div> -->

    <!-- Main Pill Container -->
    <form @submit.prevent="handleSubmit"
      class="w-full bg-neutral-900/95 backdrop-blur-md border border-neutral-800 rounded-3xl p-3 shadow-2xl flex flex-col gap-2">
      <!-- Top Input Row -->
      <div class="px-2 pt-1">
        <input v-model="inputPrompt" type="text" placeholder="Command anything try /help" :disabled="isProcessing"
          class="w-full bg-transparent text-neutral-200 placeholder-neutral-500 text-sm outline-none border-0 focus:ring-0 disabled:opacity-50" />
      </div>

      <!-- Bottom Action Row -->
      <div class="flex items-center justify-between text-xs text-neutral-400 px-1 pt-1">
        <!-- Left Pill Actions -->
        <div class="flex items-center gap-1.5 overflow-x-auto">
          <!-- Add / Attach Button -->
          <button type="button" class="p-1 rounded-md hover:bg-neutral-800 hover:text-neutral-200 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 5v14m-7-7h14" />
            </svg>
          </button>
        </div>

        <!-- Right Action Controls -->
        <div class="flex items-center gap-2">
          <!-- Voice / Mic Button -->
          <button type="button"
            class="p-1 rounded-md hover:bg-neutral-800 text-neutral-400 hover:text-neutral-200 transition-colors">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round"
                d="M12 18.75a6 6 0 0 0 6-6v-1.5m-6 7.5a6 6 0 0 1-6-6v-1.5m6 7.5v3.75m-3.75 0h7.5M12 15a3 3 0 0 0 3-3V6a3 3 0 0 0-6 0v6a3 3 0 0 0 3 3Z" />
            </svg>
          </button>

          <!-- Circular Green Submit Button -->
          <button type="submit" :disabled="isProcessing || !inputPrompt.trim()"
            class="w-7 h-7 rounded-full bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 text-white flex items-center justify-center transition-all disabled:opacity-40 disabled:hover:bg-emerald-600 disabled:cursor-not-allowed shadow-md">
            <!-- Loading Spinner -->
            <svg v-if="isProcessing" class="w-3.5 h-3.5 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8v8H4z" />
            </svg>

            <!-- Arrow Up Icon -->
            <svg v-else class="w-4 h-4" fill="none" stroke="currentColor" stroke-width="2.5" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 10.5 12 3m0 0 7.5 7.5M12 3v18" />
            </svg>
          </button>
        </div>
      </div>
    </form>
  </aside>
</template>