<script setup lang="ts">
import { clock } from '@/config/flight'
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
        class="relative min-w-24 flex-1 overflow-hidden bg-[#0a0a0a]"
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
          class="absolute inset-y-3 border border-white/20 bg-white/5"
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
          class="absolute inset-y-4 min-w-1 cursor-pointer border-l border-white/40 bg-white/15 hover:bg-white/30"
          :class="
            failed(step.phase)
              ? '!bg-[#d71921]/40'
              : step.phase === 'cancelled'
                ? '!bg-zinc-600/40'
                : ''
          "
          :style="{
            left: `${(step.start / timeline.total) * 100}%`,
            width: `${(Math.max(0, step.end - step.start) / timeline.total) * 100}%`,
          }"
          :aria-label="`${task.title}, step ${index + 1}: ${step.title}, ${step.phase}`"
          :title="`${step.title} · ${step.phase}`"
          @click.stop="emit('seek', step.start)"
        />
        <span
          class="pointer-events-none absolute inset-y-0 w-px bg-[#f1f1ef]"
          :style="{ left: `${(timeMs / timeline.total) * 100}%` }"
          data-task-playhead=""
        />
      </div>
    </div>
  </section>
</template>