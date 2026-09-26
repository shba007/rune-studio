<script setup lang="ts">
import { useMediaStore } from '@/stores/mediaStore'

const mediaStore = useMediaStore()

function formatBytes(bytes: number): string {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}
</script>

<template>
  <div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0c0c0e]">
    <!-- Gallery Filter Toolbar -->
    <div
      class="p-3.5 border-b border-[#1f1f26] flex items-center justify-between gap-4 bg-[#121217]"
    >
      <div class="flex items-center gap-1 bg-[#17171e] border border-[#272732] rounded-xl p-1">
        <button
          type="button"
          @click="mediaStore.filterType = 'all'"
          class="px-3 py-1 text-xs font-medium rounded-lg transition-colors"
          :class="
            mediaStore.filterType === 'all'
              ? 'bg-[#242430] text-white shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          "
        >
          All ({{ mediaStore.mediaItems.length }})
        </button>
        <button
          type="button"
          @click="mediaStore.filterType = 'image'"
          class="px-3 py-1 text-xs font-medium rounded-lg transition-colors"
          :class="
            mediaStore.filterType === 'image'
              ? 'bg-[#242430] text-white shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          "
        >
          Images
        </button>
        <button
          type="button"
          @click="mediaStore.filterType = 'video'"
          class="px-3 py-1 text-xs font-medium rounded-lg transition-colors"
          :class="
            mediaStore.filterType === 'video'
              ? 'bg-[#242430] text-white shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200'
          "
        >
          Videos
        </button>
      </div>

      <div class="relative w-64">
        <input
          v-model="mediaStore.searchQuery"
          type="text"
          placeholder="Filter assets by name..."
          class="w-full bg-[#17171e] border border-[#272732] rounded-xl pl-8 pr-3 py-1 text-xs text-neutral-200 placeholder-neutral-500 outline-none focus:border-emerald-600 transition-colors"
        />
        <svg
          class="w-3.5 h-3.5 text-neutral-500 absolute left-2.5 top-1/2 -translate-y-1/2"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"
          />
        </svg>
      </div>
    </div>

    <!-- Gallery Grid -->
    <div class="flex-1 overflow-y-auto p-4">
      <div
        v-if="mediaStore.filteredMedia.length === 0"
        class="h-full min-h-[300px] flex flex-col items-center justify-center border-2 border-dashed border-[#22222a] rounded-3xl text-neutral-500 gap-3"
      >
        <svg
          class="w-10 h-10 opacity-30"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          viewBox="0 0 24 24"
        >
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            d="m2.25 15.75 5.159-5.159a2.25 2.25 0 0 1 3.182 0l5.159 5.159m-1.5-1.5 1.409-1.409a2.25 2.25 0 0 1 3.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 0 0 1.5-1.5V6a1.5 1.5 0 0 0-1.5-1.5H3.75A1.5 1.5 0 0 0 2.25 6v12a1.5 1.5 0 0 0 1.5 1.5Z"
          />
        </svg>
        <span class="text-xs">No media assets found in this folder.</span>
      </div>

      <div
        v-else
        class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3"
      >
        <div
          v-for="item in mediaStore.filteredMedia"
          :key="item.id"
          @click="mediaStore.selectMedia(item)"
          style="content-visibility: auto; contain-intrinsic-size: 180px; contain: content"
          class="group relative aspect-square bg-[#141419] rounded-2xl overflow-hidden border cursor-pointer select-none flex flex-col transition-colors duration-150"
          :class="[
            mediaStore.selectedMedia?.id === item.id
              ? 'border-blue-500 ring-2 ring-blue-500/50 shadow-lg shadow-blue-950/50'
              : 'border-[#22222c] hover:border-[#383848]',
          ]"
        >
          <div
            class="relative flex-1 w-full bg-[#0a0a0d] overflow-hidden flex items-center justify-center"
          >
            <!-- Universal Fast Thumbnail -->
            <img
              :src="item.thumbnailUrl"
              :alt="item.name"
              loading="lazy"
              decoding="async"
              class="w-full h-full object-cover will-change-transform group-hover:scale-105 transition-transform duration-200"
            />

            <!-- Video Player Overlay Chip -->
            <div
              v-if="item.mediaType === 'video'"
              class="absolute inset-0 flex items-center justify-center bg-black/25 group-hover:bg-black/15 transition-colors pointer-events-none"
            >
              <div
                class="w-8 h-8 rounded-full bg-black/65 backdrop-blur-sm border border-white/20 flex items-center justify-center shadow-lg"
              >
                <svg class="w-4 h-4 text-white fill-current ml-0.5" viewBox="0 0 24 24">
                  <path d="M8 5v14l11-7z" />
                </svg>
              </div>
              <div
                class="absolute bottom-2 left-2 px-1.5 py-0.5 bg-black/85 backdrop-blur-md rounded text-[10px] font-mono text-neutral-300 flex items-center gap-1 border border-white/10"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                VID
              </div>
            </div>

            <!-- Visual Assessment Cached Badge -->
            <div
              v-if="mediaStore.artifactsCache[item.path]"
              class="absolute top-2 left-2 px-1.5 py-0.5 bg-emerald-950/90 border border-emerald-700/80 rounded text-[9px] font-mono text-emerald-300 flex items-center gap-1 shadow-sm"
            >
              <span>★</span>
              <span>{{
                mediaStore.artifactsCache[item.path]?.fidelityScore?.toFixed(1) ?? 'PROFILED'
              }}</span>
            </div>
          </div>

          <div class="p-2 bg-[#141419] border-t border-[#1f1f28] flex flex-col gap-0.5">
            <span class="text-[11px] font-medium text-neutral-200 truncate" :title="item.name">
              {{ item.name }}
            </span>
            <span class="text-[10px] font-mono text-neutral-500">
              {{ formatBytes(item.size) }}
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>