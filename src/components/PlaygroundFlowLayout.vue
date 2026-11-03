<script setup lang="ts">
import { useElementSize } from '@vueuse/core'
import { computed, ref } from 'vue'
import { FLOW_VIEWS } from '@/config/flows'
import PlaygroundLoading from './PlaygroundLoading.vue'
import PlaygroundSettingsPanel from './PlaygroundSettingsPanel.vue'
import RuneButton from './RuneButton.vue'
import RuneChoice from './RuneChoice.vue'

const props = defineProps<{
  title: string
  settingsLabel: string
  resultLabel: string
  graphTestId: string
  appTestId: string
  running: boolean
  started: number
  loadingLabel: string
  error?: string
  stale?: boolean
  steps?: number
  graphWide?: boolean
  graphWidth?: number
  appTop?: boolean
}>()
defineEmits<{ cancel: [] }>()
const mode = defineModel<'app' | 'graph'>({ required: true })
const viewport = ref<HTMLElement | null>(null)
const { width: viewportWidth } = useElementSize(viewport)
const zoomFactor = ref(1)
const graphWidth = computed(() => props.graphWidth ?? (props.graphWide ? 1100 : 440))
const fitZoom = computed(() =>
  viewportWidth.value > 48 ? Math.min(1, (viewportWidth.value - 48) / graphWidth.value) : 1,
)
const zoom = computed(() => Math.max(0.1, Math.min(2, fitZoom.value * zoomFactor.value)))
function changeZoom(multiplier: number) {
  zoomFactor.value = Math.max(0.1, Math.min(2, zoom.value * multiplier)) / fitZoom.value
}
function onWheel(event: WheelEvent) {
  if (mode.value !== 'graph' || !event.ctrlKey) return
  event.preventDefault()
  changeZoom(event.deltaY < 0 ? 1.1 : 1 / 1.1)
}
</script>
<template>
  <div class="flex min-h-0 min-w-0 flex-1">
    <PlaygroundSettingsPanel :aria-label="settingsLabel">
      <div class="flex min-h-0 flex-1 flex-col gap-6 overflow-y-auto px-6 py-5">
        <slot name="settings" />
      </div>
    </PlaygroundSettingsPanel>
    <section
      class="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-rune-surface"
      :aria-label="resultLabel"
    >
      <header
        class="flex shrink-0 items-center justify-between border-b border-rune-text-3/20 px-5 py-3"
      >
        <span class="text-rune-caption text-rune-text-3">{{ title }}</span
        ><RuneChoice v-model="mode" label="Flow view" :options="FLOW_VIEWS" />
      </header>
      <div
        ref="viewport"
        class="flex min-h-0 flex-1 flex-col items-center overflow-auto bg-[radial-gradient(circle,rgb(255_255_255/0.16)_1px,transparent_1.5px)] bg-size-[24px_24px] px-6 py-8"
        @wheel="onWheel"
      >
        <p
          v-if="stale && !running"
          class="mb-5 w-full max-w-[440px] rounded-rune-tile border border-amber-400/20 bg-amber-400/10 p-3 text-rune-caption text-amber-200"
        >
          Inputs or models have changed. The displayed output is from the previous run.
        </p>
        <div
          v-if="mode === 'graph'"
          class="flex w-max min-w-full shrink-0 justify-center self-start"
        >
          <div
            class="flex shrink-0 flex-col items-center"
            :style="{ width: `${graphWidth}px`, zoom }"
            :data-testid="graphTestId"
          >
            <div class="mb-10 flex w-full items-end justify-between gap-3">
              <div>
                <p class="m-0 text-rune-caption text-rune-text-3 uppercase">Flow overview</p>
                <p class="mt-1 mb-0 text-rune-body text-rune-text-2">{{ title }}</p>
              </div>
              <span
                class="rounded-full border border-white/10 px-3 py-1 text-rune-caption text-rune-text-3"
                >{{ steps ?? 3 }} steps · Read only</span
              >
            </div>
            <slot name="graph" />
          </div>
        </div>
        <div
          v-else
          class="flex min-h-full w-full flex-col items-center gap-4"
          :class="appTop ? 'max-w-4xl justify-start' : 'max-w-2xl justify-center'"
          :data-testid="appTestId"
        >
          <PlaygroundLoading
            v-if="running"
            :started-at="started"
            :label="loadingLabel"
            :cancellable="true"
            @cancel="$emit('cancel')"
          />
          <p v-if="error" role="alert" class="text-center text-error">{{ error }}</p>
          <slot name="app" />
        </div>
        <p v-if="mode === 'graph' && error" role="alert" class="text-center text-error">
          {{ error }}
        </p>
      </div>
      <div
        v-if="mode === 'graph'"
        class="absolute right-4 bottom-4 z-20 flex items-center gap-1 rounded-rune-tile border border-white/10 bg-rune-sunken p-1 shadow-xl"
        aria-label="Graph zoom"
      >
        <RuneButton variant="ghost" size="sm" aria-label="Zoom out" @click="changeZoom(1 / 1.2)">
          −
        </RuneButton>
        <RuneButton variant="ghost" size="sm" aria-label="Fit graph" @click="zoomFactor = 1">
          {{ Math.round(zoom * 100) }}%
        </RuneButton>
        <RuneButton variant="ghost" size="sm" aria-label="Zoom in" @click="changeZoom(1.2)">
          +
        </RuneButton>
      </div>
      <div
        v-if="mode === 'graph' && running"
        class="absolute top-16 right-4 z-20 rounded-rune-tile border border-white/10 bg-rune-sunken px-4 py-3 shadow-xl"
        data-testid="graph-progress"
      >
        <PlaygroundLoading
          :started-at="started"
          :label="loadingLabel"
          :cancellable="true"
          compact
          @cancel="$emit('cancel')"
        />
      </div>
    </section>
  </div>
</template>