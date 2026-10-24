<script setup lang="ts">
import '@videojs/html/video/player'
import '@videojs/html/ui/container'

import { useElementSize } from '@vueuse/core'
import { computed, onBeforeUnmount, ref, watch } from 'vue'

import CursorGlyph from './CursorGlyph.vue'
import { fitBox, type ActionLayer } from '@/config/flight'

const props = defineProps<{
  videoSrc: string | null
  still: string | null
  stillMissing: boolean
  layer: ActionLayer
  layerOn: boolean
  cursor: { x: number; y: number } | null
  badge: string
  note: string | null
}>()
const markerCursor = computed(
  () =>
    props.cursor ??
    (props.layer.kind === 'click' || props.layer.kind === 'scroll'
      ? { x: props.layer.x, y: props.layer.y }
      : null),
)
const emit = defineEmits<{
  time: [ms: number]
  playing: [playing: boolean]
  duration: [ms: number]
  failed: [message: string]
}>()

interface PlayerStore {
  paused: boolean
  play: () => Promise<void>
  pause: () => void
  seek: (seconds: number) => Promise<number>
  setPlaybackRate: (rate: number) => void
}
interface PlayerElement extends HTMLElement {
  store?: PlayerStore
}

const frame = ref<HTMLElement | null>(null)
const player = ref<PlayerElement | null>(null)
const video = ref<HTMLVideoElement | null>(null)
const { width, height } = useElementSize(frame)
const aspect = ref(16 / 9)
const box = computed(() => fitBox(width.value, height.value, aspect.value))
const ready = ref(false)
let frameRequest = 0

watch(
  () => props.videoSrc,
  () => {
    ready.value = false
  },
)

function onMetadata() {
  const element = video.value
  if (!element) return
  if (element.videoWidth > 0 && element.videoHeight > 0) {
    aspect.value = element.videoWidth / element.videoHeight
  }
  ready.value = true
  emit('duration', Math.round(element.duration * 1000))
}

function onStill(event: Event) {
  const image = event.target as HTMLImageElement
  if (image.naturalWidth > 0 && image.naturalHeight > 0) {
    aspect.value = image.naturalWidth / image.naturalHeight
  }
}

function tick() {
  const element = video.value
  if (element) emit('time', Math.round(element.currentTime * 1000))
  frameRequest = element && !element.paused && !element.ended ? requestAnimationFrame(tick) : 0
}

function onPlay() {
  emit('playing', true)
  if (!frameRequest) frameRequest = requestAnimationFrame(tick)
}
function onPause() {
  emit('playing', false)
  emit('time', Math.round((video.value?.currentTime ?? 0) * 1000))
}
function onError() {
  const code = video.value?.error?.code
  emit('failed', code === 4 ? 'This video cannot be played here.' : 'The video could not be read.')
}

onBeforeUnmount(() => {
  if (frameRequest) cancelAnimationFrame(frameRequest)
})

async function store(): Promise<PlayerStore | null> {
  await customElements.whenDefined('video-player')
  return player.value?.store ?? null
}

defineExpose({
  async play() {
    const state = await store()
    if (state) await state.play()
    else await video.value?.play()
  },
  async pause() {
    const state = await store()
    if (state) state.pause()
    else video.value?.pause()
  },
  async seek(ms: number) {
    const seconds = Math.max(0, ms / 1000)
    const state = await store()
    if (state) await state.seek(seconds)
    else if (video.value) video.value.currentTime = seconds
    emit('time', Math.round(seconds * 1000))
  },
  async setRate(rate: number) {
    const state = await store()
    if (state) state.setPlaybackRate(rate)
    else if (video.value) video.value.playbackRate = rate
  },
  async fullscreen() {
    if (document.fullscreenElement) await document.exitFullscreen()
    else await frame.value?.requestFullscreen()
  },
})
</script>

<template>
  <div
    ref="frame"
    class="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden bg-black"
    data-testid="stage"
  >
    <div
      v-if="box.width > 0"
      class="relative overflow-hidden bg-[#0a0a0a]"
      :style="{ width: `${box.width}px`, height: `${box.height}px` }"
    >
      <video-player v-if="videoSrc" ref="player" class="block size-full">
        <media-container class="block size-full">
          <video
            ref="video"
            :src="videoSrc"
            class="block size-full"
            muted
            playsinline
            preload="auto"
            data-testid="video"
            aria-label="Session video"
            @loadedmetadata="onMetadata"
            @play="onPlay"
            @pause="onPause"
            @seeked="tick"
            @error="onError"
          />
        </media-container>
      </video-player>
      <img
        v-else-if="still && !stillMissing"
        :src="still"
        alt="Screenshot of the screen at this step"
        class="block size-full object-contain"
        data-testid="still"
        @load="onStill"
      />
      <p
        v-else
        class="m-0 flex size-full items-center justify-center p-6 text-center text-body text-zinc-500"
        data-testid="no-picture"
      >
        {{ stillMissing ? 'This screenshot was deleted.' : 'There is no picture for this step.' }}
      </p>

      <div
        v-if="layerOn"
        class="pointer-events-none absolute inset-0"
        data-testid="layer"
        aria-hidden="true"
      >
        <CursorGlyph
          v-if="markerCursor"
          class="absolute transition-[left,top] duration-300 ease-out"
          :kind="layer.kind === 'type' ? 'text' : 'default'"
          :style="{
            left: `calc(${markerCursor.x / 10}% - ${layer.kind === 'type' ? 16 : 1}px)`,
            top: `calc(${markerCursor.y / 10}% - ${layer.kind === 'type' ? 16 : 1}px)`,
          }"
          :ripple="layer.kind === 'click' || layer.kind === 'scroll'"
          data-testid="cursor"
        />
        <template v-if="layer.kind === 'click' || layer.kind === 'scroll'">
          <span
            class="absolute -translate-x-1/2 translate-y-5 rounded-full bg-black/70 px-2 text-caption text-zinc-100"
            :style="{ left: `${layer.x / 10}%`, top: `${layer.y / 10}%` }"
            >{{ layer.label }}</span
          >
        </template>
        <p
          v-else-if="layer.kind === 'type' || layer.kind === 'keys'"
          class="absolute bottom-6 left-1/2 m-0 max-w-[80%] -translate-x-1/2 truncate rounded-full bg-black/75 px-4 py-1 text-body text-zinc-100"
          data-testid="typed"
        >
          {{ layer.text }}
        </p>
        <span
          v-if="badge"
          class="absolute top-3 left-3 rounded-full bg-black/70 px-2 text-caption text-zinc-200"
          data-testid="badge"
          >{{ badge }}</span
        >
      </div>
    </div>
    <p
      v-if="note"
      class="absolute bottom-2 left-1/2 m-0 -translate-x-1/2 rounded-full bg-black/70 px-3 text-label text-zinc-400"
      data-testid="note"
    >
      {{ note }}
    </p>
  </div>
</template>