<script setup lang="ts">
import { computed, ref } from 'vue'

import type { Agent } from '@/composables/useAgent'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const down = computed(() => agent.health.value.tone === 'down')
const address = computed(() => agent.settings.value?.baseUrl ?? '')
const checking = ref(false)

async function recheck() {
  checking.value = true
  try {
    await agent.refreshBackend()
  } finally {
    checking.value = false
  }
}
</script>

<template>
  <section
    class="flex w-full flex-col gap-2 rounded-rune-panel border bg-rune-surface p-rune-2 font-rune-reading text-zinc-100"
    :class="down ? 'border-attention/50' : 'border-rune-outline'"
    role="status"
    aria-label="Model server"
  >
    <header class="flex h-6 items-center gap-2">
      <span class="size-2 shrink-0 rounded-full" :class="down ? 'bg-attention' : 'bg-primary'" />
      <span class="text-rune-caption uppercase" :class="down ? 'text-attention' : 'text-zinc-300'">
        {{ down ? 'Model server not reachable' : 'Model server needs attention' }}
      </span>
    </header>
    <p v-if="down" class="text-rune-body">Start your model server, or change its address.</p>
    <p v-else class="text-rune-body">{{ agent.health.value.title }}</p>
    <p
      v-if="address"
      class="m-0 truncate text-rune-caption text-zinc-500"
      :title="address"
      data-testid="server-address"
    >
      {{ address }}
    </p>
    <div class="mt-1 flex gap-2">
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full border border-white/25 text-rune-caption text-zinc-200 uppercase transition hover:bg-white/10 disabled:cursor-default disabled:opacity-40"
        :disabled="checking"
        @click="recheck"
      >
        Check again
      </button>
      <button
        type="button"
        class="h-8 flex-1 cursor-pointer rounded-full bg-primary text-rune-caption text-[#111] uppercase transition hover:bg-white"
        @click="agent.openSettings('models')"
      >
        Models
      </button>
    </div>
  </section>
</template>