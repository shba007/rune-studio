<script setup lang="ts">
import { clock } from '@/config/flight'
import AppIcon from './AppIcon.vue'
import type { TaskTimeline } from '@/config/taskTimeline'
defineProps<{ timeline: TaskTimeline; timeMs: number }>()
const emit = defineEmits<{ seek: [ms: number] }>()
function seek(event: MouseEvent, total: number) {
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect()
  if (rect.width > 0)
    emit('seek', Math.max(0, Math.min(total, ((event.clientX - rect.left) / rect.width) * total)))
}
const failed = (phase: string) => ['failed', 'blocked'].includes(phase)
</script>
<template>
  <section
    aria-label="Task timelines"
    class="mx-4 my-3 max-h-64 shrink-0 overflow-auto rounded-2xl border border-white/15"
  >
    <div
      v-for="task in timeline.rows"
      :key="task.id"
      class="flex min-h-16 border-b border-white/10 last:border-b-0"
      :data-task-row="task.id"
    >
      <div class="w-44 shrink-0 border-r border-white/10 px-3 py-2">
        <p class="m-0 line-clamp-2 text-label text-zinc-100" :title="task.title">
          {{ task.title }}
        </p>
        <p
          class="m-0 text-caption"
          :class="failed(task.phase) ? 'text-[#ff6b72]' : 'text-zinc-400'"
        >
          {{ task.kind === 'desktop' ? 'Desktop' : 'Reasoning' }} · {{ task.phase }}
        </p>
      </div>
      <div
        class="relative min-h-20 min-w-24 flex-1 overflow-hidden bg-[#101010]"
        role="slider"
        tabindex="0"
        :aria-label="`Seek ${task.title}`"
        :aria-valuemin="0"
        :aria-valuemax="timeline.total"
        :aria-valuenow="Math.round(timeMs)"
        :aria-valuetext="clock(timeMs)"
        @click="seek($event, timeline.total)"
        @keydown.left.prevent="emit('seek', Math.max(0, timeMs - 1000))"
        @keydown.right.prevent="emit('seek', Math.min(timeline.total, timeMs + 1000))"
      >
        <div
          class="absolute inset-y-0 border-l border-white/10"
          :class="failed(task.phase) ? '!border-[#d71921]/50 !bg-[#d71921]/10' : ''"
          :style="{
            left: `${(task.start / timeline.total) * 100}%`,
            width: `${((task.end - task.start) / timeline.total) * 100}%`,
          }"
          data-task-span=""
        />
        <button
          v-for="(step, index) in task.steps"
          :key="index"
          type="button"
          class="absolute inset-y-0 min-w-1 cursor-pointer overflow-hidden border-l border-white/10 text-zinc-200 hover:bg-white/[0.07]"
          :class="
            failed(step.phase)
              ? '!text-[#ff6b72]'
              : step.phase === 'cancelled'
                ? '!text-zinc-500'
                : ''
          "
          :style="{
            left: `${(step.start / timeline.total) * 100}%`,
            width: `${(Math.max(0, step.end - step.start) / timeline.total) * 100}%`,
          }"
          :aria-label="`${task.title}, step ${index + 1}: ${step.title}, ${step.phase}`"
          :title="`${step.title} · ${step.phase} · ${clock(Math.max(0, step.end - step.start))}`"
          @click.stop="emit('seek', step.start)"
        >
          <AppIcon
            :name="task.kind === 'desktop' ? 'monitor' : 'thinking'"
            size="sm"
            class="absolute top-2 left-1.5 opacity-80"
          />
          <span class="absolute top-2 right-1.5 text-caption tabular-nums" data-step-duration>{{
            clock(Math.max(0, step.end - step.start))
          }}</span>
          <span class="pointer-events-none absolute right-0 bottom-3 left-0 h-3 timeline-dots" />
        </button>
        <span
          class="pointer-events-none absolute inset-y-0 w-px bg-[#f1f1ef]"
          :style="{ left: `${(timeMs / timeline.total) * 100}%` }"
          data-task-playhead=""
        />
      </div>
    </div>
  </section>
</template>
<style scoped>
.timeline-dots {
  background-image: radial-gradient(circle, currentColor 1.1px, transparent 1.6px);
  background-size: 5px 5px;
  opacity: 0.65;
}
</style>