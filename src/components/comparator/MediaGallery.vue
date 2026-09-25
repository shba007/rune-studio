<script setup lang="ts">
import { useMediaStore } from '@/stores/mediaStore'
import type { MediaItem } from '@/types/media'

const mediaStore = useMediaStore()

function formatBytes(bytes: number): string {
  if (!bytes) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`
}

function getItemBadge(item: MediaItem): 'A' | 'B' | null {
  if (mediaStore.selectedItemA?.id === item.id) return 'A'
  if (mediaStore.selectedItemB?.id === item.id) return 'B'
  return null
}
</script>

<template>
  <div class="flex-1 flex flex-col h-full overflow-hidden bg-[#0c0c0e]">
    <!-- Filter & Search Toolbar -->
    <div
      class="p-3.5 border-b border-[#1f1f26] flex items-center justify-between gap-4 bg-[#121217]"
    >
      <!-- Media Type Filters -->
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

      <!-- Search Input -->
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

    <!-- Gallery Grid Area -->
    <div class="flex-1 overflow-y-auto p-4">
      <!-- Empty State -->
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
        <span class="text-xs">No media assets found. Select a folder and click Scan.</span>
      </div>

      <!-- Grid Cards -->
      <div
        v-else
        class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3"
      >
        <div
          v-for="item in mediaStore.filteredMedia"
          :key="item.id"
          @click="mediaStore.selectMedia(item)"
          class="group relative aspect-square bg-[#141419] rounded-2xl overflow-hidden border cursor-pointer transition-all duration-200 select-none flex flex-col"
          :class="[
            getItemBadge(item) === 'A'
              ? 'border-blue-500 ring-2 ring-blue-500/40 shadow-lg shadow-blue-950/50'
              : getItemBadge(item) === 'B'
                ? 'border-emerald-500 ring-2 ring-emerald-500/40 shadow-lg shadow-emerald-950/50'
                : 'border-[#22222c] hover:border-[#353545] hover:shadow-md',
          ]"
        >
          <!-- Media Preview Thumbnail -->
          <div
            class="relative flex-1 w-full bg-[#0a0a0d] overflow-hidden flex items-center justify-center"
          >
            <img
              v-if="item.mediaType === 'image'"
              :src="item.thumbnailUrl"
              :alt="item.name"
              loading="lazy"
              class="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
            />
            <video
              v-else-if="item.mediaType === 'video'"
              :src="item.thumbnailUrl"
              muted
              preload="metadata"
              class="w-full h-full object-cover"
            />

            <!-- Video Icon Indicator -->
            <div
              v-if="item.mediaType === 'video'"
              class="absolute bottom-2 left-2 px-1.5 py-0.5 bg-black/80 backdrop-blur-md rounded text-[10px] font-mono text-neutral-300 flex items-center gap-1"
            >
              <svg class="w-2.5 h-2.5 text-emerald-400" fill="currentColor" viewBox="0 0 24 24">
                <path d="M8 5v14l11-7z" />
              </svg>
              VID
            </div>

            <!-- Selection Badges -->
            <div
              v-if="getItemBadge(item) === 'A'"
              class="absolute top-2 right-2 w-6 h-6 rounded-full bg-blue-600 text-white font-bold text-xs flex items-center justify-center shadow-lg border border-blue-400"
            >
              A
            </div>
            <div
              v-else-if="getItemBadge(item) === 'B'"
              class="absolute top-2 right-2 w-6 h-6 rounded-full bg-emerald-600 text-white font-bold text-xs flex items-center justify-center shadow-lg border border-emerald-400"
            >
              B
            </div>
          </div>

          <!-- Card Footer Metadata -->
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