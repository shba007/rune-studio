<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { useIntervalFn, useNow } from '@vueuse/core'
import { computed, onMounted, useTemplateRef } from 'vue'

import { clock, secondsLeft } from '@/config/reflection'
import type { Agent } from '@/composables/useAgent'

const props = defineProps<{ agent: Agent }>()
const agent = props.agent

const now = useNow({ scheduler: (update) => useIntervalFn(update, 250) })
const left = computed(() => {
  const ends = agent.reflectionPrompt.value?.endsAt
  return ends === undefined ? 0 : secondsLeft(now.value.getTime(), ends)
})

const deny = useTemplateRef<InstanceType<typeof RuneButton>>('deny')
onMounted(() => deny.value?.$el.focus())
</script>

<template>
  <section
    v-if="agent.reflectionPrompt.value"
    class="flex w-full flex-col gap-2 rounded-rune-panel bg-rune-surface p-rune-2 font-rune-reading text-rune-ink"
    role="alertdialog"
    aria-label="Review recent tasks"
  >
    <header class="flex h-6 items-center gap-2">
      <span class="size-2 shrink-0 rounded-full bg-primary" />
      <span class="text-rune-caption text-rune-text-2 uppercase">Rune</span>
      <span class="ml-auto text-rune-caption text-rune-text-2 uppercase" aria-live="off">
        Starts in {{ clock(left) }}
      </span>
    </header>
    <p class="text-rune-body">Rune wants to review its recent tasks.</p>
    <div class="mt-1 flex gap-2">
      <RuneButton size="sm" ref="deny" class="flex-1" @click="agent.answerReflection(false)">
        Deny
      </RuneButton>
      <RuneButton variant="primary" size="sm" class="flex-1" @click="agent.answerReflection(true)">
        Start
      </RuneButton>
    </div>
  </section>
</template>