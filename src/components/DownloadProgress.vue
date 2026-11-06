<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'

const props = defineProps<{
  phase: string
  filename?: string
  downloaded: number
  total?: number | null
}>()
const speed = ref<number | null>(null)
let previous: { phase: string; filename?: string; bytes: number; at: number } | undefined
let staleTimer: ReturnType<typeof setTimeout> | undefined
watch(
  () => [props.phase, props.filename, props.downloaded] as const,
  ([phase, filename, bytes]) => {
    const now = Date.now()
    if (
      !previous ||
      previous.phase !== phase ||
      previous.filename !== filename ||
      bytes < previous.bytes
    ) {
      speed.value = null
    } else if (now > previous.at) {
      const current = ((bytes - previous.bytes) * 1000) / (now - previous.at)
      speed.value =
        current > 0 ? (speed.value == null ? current : speed.value * 0.65 + current * 0.35) : null
    }
    previous = { phase, filename, bytes, at: now }
    clearTimeout(staleTimer)
    staleTimer = setTimeout(() => {
      speed.value = null
    }, 3000)
  },
  { immediate: true },
)
onBeforeUnmount(() => clearTimeout(staleTimer))
const fraction = computed(() =>
  props.total && props.total > 0 ? Math.min(1, Math.max(0, props.downloaded / props.total)) : null,
)
const size = (n: number) =>
  n >= 1_000_000_000 ? `${(n / 1_000_000_000).toFixed(2)} GB` : `${(n / 1_000_000).toFixed(1)} MB`
const eta = computed(() => {
  if (!props.total || !speed.value || props.downloaded >= props.total) return null
  const seconds = Math.ceil((props.total - props.downloaded) / speed.value)
  return seconds >= 3600
    ? `${Math.floor(seconds / 3600)}h ${Math.ceil((seconds % 3600) / 60)}m`
    : seconds >= 60
      ? `${Math.ceil(seconds / 60)}m`
      : `${seconds}s`
})
</script>

<template>
  <div class="flex flex-col gap-2 text-rune-caption text-rune-text-2" role="status">
    <span
      >{{ phase === 'engine' ? 'Preparing engine' : 'Downloading model'
      }}<template v-if="filename"> · {{ filename }}</template></span
    >
    <div class="flex flex-wrap items-center gap-2">
      <strong class="text-rune-ink">{{
        fraction == null ? 'Preparing…' : `${Math.round(fraction * 100)}%`
      }}</strong>
      <span
        class="download-dots"
        role="progressbar"
        :aria-label="phase === 'engine' ? 'Engine download' : 'Model download'"
        :aria-valuenow="fraction == null ? undefined : Math.round(fraction * 100)"
        aria-valuemin="0"
        aria-valuemax="100"
      >
        <span
          v-for="n in 20"
          :key="n"
          :class="{ filled: fraction != null && n <= Math.round(fraction * 20) }"
        />
      </span>
      <span v-if="total">{{ size(downloaded) }} / {{ size(total) }} · current file</span>
      <span v-if="speed">{{ size(speed) }}/s</span>
      <span v-if="eta">{{ eta }} remaining</span>
    </div>
  </div>
</template>

<style scoped>
.download-dots {
  display: inline-grid;
  grid-template-columns: repeat(10, 7px);
  gap: 3px;
}
.download-dots span {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-rune-raised);
}
.download-dots .filled {
  background: var(--accent);
}
</style>