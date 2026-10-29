<script setup lang="ts">
import RuneButton from './RuneButton.vue'
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
    class="flex w-full flex-col gap-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    :class="down ? '' : ''"
    role="status"
    aria-label="Model server"
  >
    <header class="flex h-6 items-center gap-2">
      <span class="size-2 shrink-0 rounded-full" :class="down ? 'bg-attention' : 'bg-primary'" />
      <span
        class="text-rune-caption uppercase"
        :class="down ? 'text-attention' : 'text-rune-text-2'"
      >
        {{ down ? 'Model server not reachable' : 'Model server needs attention' }}
      </span>
    </header>
    <p v-if="down" class="text-rune-body">Start your model server, or change its address.</p>
    <p v-else class="text-rune-body">{{ agent.health.value.title }}</p>
    <p
      v-if="address"
      class="m-0 truncate text-rune-caption text-rune-text-2"
      :title="address"
      data-testid="server-address"
    >
      {{ address }}
    </p>
    <div class="mt-1 flex gap-2">
      <RuneButton size="sm" class="flex-1" :disabled="checking" @click="recheck">
        Check again
      </RuneButton>
      <RuneButton variant="primary" size="sm" class="flex-1" @click="agent.openSettings('models')">
        Models
      </RuneButton>
    </div>
  </section>
</template>