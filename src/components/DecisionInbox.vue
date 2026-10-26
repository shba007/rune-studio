<script setup lang="ts">
import { computed, ref } from 'vue'
import { useEventListener } from '@vueuse/core'
import { useDecisions, type DecisionModel } from '@/composables/useDecisions'
import DecisionCard from './DecisionCard.vue'
const props = defineProps<{ compact?: boolean; goalsOnly?: boolean; model?: DecisionModel }>()
const decisions = props.model ?? useDecisions()
// The floating window fits its content, so a viewport-relative cap feeds back
// into its own measured height. Screen work-area dimensions stay independent.
const compactHeight = ref(0)
function updateCompactHeight() {
  compactHeight.value = Math.max(80, Math.min(520, (window.screen.availHeight || 720) * 0.9 - 180))
}
updateCompactHeight()
useEventListener(window, 'resize', updateCompactHeight)
const visible = computed(() =>
  props.compact
    ? decisions.current.value
      ? [decisions.current.value]
      : []
    : decisions.entries.value.filter((e) => (props.goalsOnly ? e.kind === 'goal' : true)),
)
</script>
<template>
  <div
    class="flex flex-col gap-3"
    :class="compact ? 'shrink-0 overflow-y-auto' : ''"
    :style="compact ? { maxHeight: `${compactHeight}px` } : undefined"
  >
    <p v-if="decisions.error.value" role="alert" class="m-0 text-label text-[#ff6b72]">
      {{ decisions.error.value }}
    </p>
    <p v-if="!compact && !visible.length" class="text-label text-zinc-400">
      No goal conversations yet. Describe an outcome in the composer to begin.
    </p>
    <article
      v-for="entry in visible"
      :key="entry.id"
      class="flex min-w-0 flex-col gap-2 [overflow-wrap:anywhere]"
    >
      <p class="m-0 text-label text-zinc-300 whitespace-pre-wrap">{{ entry.text }}</p>
      <p v-if="entry.error" role="alert" class="m-0 text-label text-[#ff6b72]">{{ entry.error }}</p>
      <div
        v-if="entry.approvedGoal && entry.question"
        class="rounded-2xl border border-white/10 p-4 text-label"
      >
        <p class="mt-0 text-caption text-zinc-400 uppercase">Approved definition</p>
        <dl class="m-0">
          <div v-for="(value, key) in entry.approvedGoal" :key="key" class="mb-2 last:mb-0">
            <dt class="text-caption text-zinc-500 uppercase">{{ key }}</dt>
            <dd class="m-0 text-zinc-200">{{ value }}</dd>
          </div>
        </dl>
        <p class="mb-0 text-caption text-zinc-400">
          Review proposed updates below before approving a replacement.
        </p>
      </div>
      <dl v-if="entry.goal" class="m-0 rounded-2xl border border-white/10 p-4 text-label">
        <div v-for="(value, key) in entry.goal" :key="key" class="mb-2 last:mb-0">
          <dt class="text-caption text-zinc-500 uppercase">{{ key }}</dt>
          <dd class="m-0 text-zinc-200 whitespace-pre-wrap">{{ value }}</dd>
        </div>
      </dl>
      <DecisionCard
        v-if="entry.question"
        :question="entry.question"
        :busy="decisions.pending.value"
        :deferable="compact"
        :cancel-label="entry.approvedGoal ? 'Cancel changes' : 'Cancel request'"
        @answer="decisions.answer"
        @defer="decisions.defer"
      />
      <p v-else class="m-0 text-label text-zinc-400">
        {{
          entry.status === 'goal_ready'
            ? 'Goal approved. Pathways and scheduling are not available yet.'
            : entry.status === 'interrupted'
              ? 'Review task history before creating another execution attempt.'
              : 'Understanding your request…'
        }}
      </p>
      <button
        v-if="entry.status === 'goal_ready'"
        type="button"
        :disabled="decisions.pending.value"
        class="self-start cursor-pointer rounded-full border border-white/20 px-4 py-2 text-label text-zinc-200 disabled:opacity-40"
        @click="decisions.refine(entry)"
      >
        Refine goal
      </button>
    </article>
  </div>
</template>