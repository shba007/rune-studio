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
    class="mx-4 my-3 max-h-64 shrink-0 overflow-auto rounded-rune-tile bg-rune-raised"
  >
    <div
      v-for="task in timeline.rows"
      :key="task.id"
      class="flex min-h-16"
      :data-task-row="task.id"
    >
      <div class="w-44 shrink-0 px-3 py-2">
        <p class="m-0 line-clamp-2 text-rune-caption text-rune-ink" :title="task.title">
          {{ task.title }}
        </p>
        <p
          class="m-0 text-rune-heading"
          :class="failed(task.phase) ? 'text-error' : 'text-rune-text-2'"
        >
          {{ task.kind === 'desktop' ? 'Desktop' : 'Reasoning' }} · {{ task.phase }}
        </p>
      </div>
      <div
        class="relative min-h-20 min-w-24 flex-1 overflow-hidden bg-rune-sunken"
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
        <button
          v-for="segment in task.preparation"
          :key="`${segment.title}-${segment.start}`"
          :data-preparation-phase="segment.title"
          type="button"
          class="absolute inset-y-0 cursor-pointer overflow-hidden bg-white/[0.025] text-left text-rune-text-2 hover:bg-white/5"
          :style="{
            left: `${(segment.start / timeline.total) * 100}%`,
            width: `${((segment.end - segment.start) / timeline.total) * 100}%`,
          }"
          :aria-label="`${task.title}: ${segment.title}`"
          :title="`${segment.title} · ${clock(segment.end - segment.start)}`"
          @click.stop="emit('seek', segment.start)"
        >
          <span class="absolute top-2 left-2 text-rune-heading">{{ segment.title }}</span
          ><span
            class="pointer-events-none absolute right-0 bottom-3 left-0 h-3 timeline-dots opacity-25"
          />
        </button>
        <div
          class="absolute inset-y-0"
          :class="failed(task.phase) ? '!bg-attention/10' : ''"
          :style="{
            left: `${(task.start / timeline.total) * 100}%`,
            width: `${((task.end - task.start) / timeline.total) * 100}%`,
          }"
          data-task-span=""
        />
        <button
          v-for="(step, index) in task.steps"
          :key="index"
          data-task-step=""
          type="button"
          class="absolute inset-y-0 min-w-1 cursor-pointer overflow-hidden text-rune-ink hover:bg-white/[0.07]"
          :class="
            failed(step.phase)
              ? '!text-error'
              : step.phase === 'cancelled'
                ? '!text-rune-text-2'
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
          <span
            class="absolute top-2 right-1.5 text-rune-heading tabular-nums"
            data-step-duration
            >{{ clock(Math.max(0, step.end - step.start)) }}</span
          >
          <span class="pointer-events-none absolute right-0 bottom-3 left-0 h-3 timeline-dots" />
        </button>
        <span
          class="pointer-events-none absolute inset-y-0 w-px bg-rune-ink"
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