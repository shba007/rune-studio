<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, ref } from 'vue'
import { inspectRawFolder, readRawFile, type RawFile } from '@/config/rawFiles'
import PlaygroundLoading from './PlaygroundLoading.vue'
import PlaygroundSettingsPanel from './PlaygroundSettingsPanel.vue'
import RuneButton from './RuneButton.vue'

const files = ref<RawFile[]>([])
const selected = ref('')
const running = ref(false)
const started = ref(0)
const error = ref('')
const saved = ref<{ path: string; width: number; height: number } | null>(null)
const chosen = computed(() =>
  files.value.find((item) => item.path === selected.value && !item.blocked),
)
const blocked = computed(() => files.value.filter((item) => item.blocked))

function chooseFolder(event: Event) {
  try {
    files.value = inspectRawFolder(Array.from((event.target as HTMLInputElement).files ?? []))
    selected.value = ''
    saved.value = null
    error.value = ''
  } catch (e) {
    files.value = []
    selected.value = ''
    saved.value = null
    error.value = String(e)
  }
}
async function develop() {
  if (!chosen.value || running.value) return
  running.value = true
  started.value = Date.now()
  error.value = ''
  saved.value = null
  try {
    saved.value = await invoke('portfolio_develop_raw', {
      raw: await readRawFile(chosen.value.file),
    })
  } catch (e) {
    error.value = String(e)
  } finally {
    running.value = false
  }
}
</script>

<template>
  <PlaygroundSettingsPanel aria-label="Develop RAW settings">
    <div class="flex min-h-0 flex-1 flex-col gap-5 overflow-y-auto px-6 py-5">
      <h1 class="m-0 text-rune-title">Develop RAW</h1>
      <p class="m-0 text-rune-body text-rune-text-2">
        Develop a camera-original Sony ARW into a full-resolution 16-bit sRGB PNG.
      </p>
      <label for="raw-develop-folder" class="text-rune-heading text-rune-text-3 uppercase"
        >Shoot folder</label
      >
      <input
        id="raw-develop-folder"
        type="file"
        webkitdirectory
        multiple
        :disabled="running"
        @change="chooseFolder"
      />
      <p class="m-0 text-rune-caption text-rune-text-3">
        {{ files.length }} RAW files · {{ blocked.length }} blocked by XMP edits
      </p>
      <label class="flex flex-col gap-2 text-rune-caption"
        >RAW image
        <select
          v-model="selected"
          :disabled="running"
          aria-label="RAW image"
          class="rounded-rune-tile bg-rune-sunken p-2"
        >
          <option value="">Select a RAW image</option>
          <option
            v-for="item in files"
            :key="item.path"
            :value="item.path"
            :disabled="item.blocked"
          >
            {{ item.path }}{{ item.blocked ? ' (XMP edits blocked)' : '' }}
          </option>
        </select>
      </label>
      <p class="m-0 text-rune-caption text-rune-text-3">
        Files with XMP edits remain blocked. The result is a developed source image without a cutout
        or camera metadata. Choose a new filename; existing files are not overwritten.
      </p>
      <RuneButton variant="primary" :disabled="!chosen || running" @click="develop"
        >Develop 16-bit PNG</RuneButton
      >
    </div>
  </PlaygroundSettingsPanel>
  <section
    class="flex min-w-0 flex-1 items-center justify-center overflow-auto bg-rune-surface p-6"
    aria-label="Develop RAW result"
  >
    <PlaygroundLoading
      v-if="running"
      :started-at="started"
      label="Developing RAW; choose a new PNG filename"
      :cancellable="false"
    />
    <p v-else-if="error" role="alert" class="text-error">{{ error }}</p>
    <p v-else-if="saved" role="status" class="break-all text-rune-body">
      Saved {{ saved.width }} × {{ saved.height }} 16-bit PNG to {{ saved.path }}
    </p>
    <p v-else class="text-rune-body text-rune-text-3">
      Choose a RAW image and develop it to a new file.
    </p>
  </section>
</template>