<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMediaStore } from '@/stores/mediaStore'
import { useMediaScanner } from '@/composables/useMediaScanner'

const route = useRoute()
const router = useRouter()
const mediaStore = useMediaStore()
const { scanDirectory } = useMediaScanner()

const isStudioActive = computed(() => route.path.startsWith('/studio'))
const isComparatorActive = computed(() => route.path.startsWith('/media-comparator'))

async function openRecentFolder(path: string) {
  mediaStore.setDirectory(path)
  if (!isComparatorActive.value) {
    await router.push('/media-comparator')
  }
  const items = await scanDirectory(path, mediaStore.isRecursive)
  mediaStore.setMediaItems(items)
}

function truncatePath(path: string): string {
  const parts = path.replace(/\\/g, '/').split('/')
  return parts.length > 2 ? `.../${parts.slice(-2).join('/')}` : path
}
</script>

<template>
  <aside
    class="w-64 h-full bg-[#101014] border-r border-[#222228] flex flex-col justify-between select-none shrink-0 z-20"
  >
    <!-- Top Brand & Navigation -->
    <div class="flex flex-col overflow-y-auto">
      <!-- App Header Brand -->
      <div class="p-4 pb-3 flex items-center justify-between border-b border-[#1c1c22]">
        <div class="flex items-center gap-2.5">
          <div
            class="w-7 h-7 rounded-xl bg-gradient-to-tr from-emerald-600 to-teal-400 flex items-center justify-center shadow-lg shadow-emerald-950/40 text-neutral-950 font-black text-sm"
          >
            R
          </div>
          <div class="flex items-center gap-1.5">
            <span class="font-bold text-sm tracking-tight text-neutral-100">rune</span>
            <span
              class="text-[9px] font-mono uppercase px-1.5 py-0.5 rounded bg-emerald-950/80 text-emerald-400 border border-emerald-800/80 font-bold"
            >
              studio
            </span>
          </div>
        </div>
      </div>

      <!-- Workspaces Navigation -->
      <div class="p-3 flex flex-col gap-1">
        <span class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500 px-2 py-1"
          >Workspaces</span
        >

        <RouterLink
          to="/media-comparator"
          class="flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium transition-all"
          :class="
            isComparatorActive
              ? 'bg-[#1e1e24] text-white shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200 hover:bg-[#16161b]'
          "
        >
          <svg
            class="w-4 h-4 text-emerald-400"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="m2.25 15.75 5.159-5.159a2.25 2.25 0 0 1 3.182 0l5.159 5.159m-1.5-1.5 1.409-1.409a2.25 2.25 0 0 1 3.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 0 0 1.5-1.5V6a1.5 1.5 0 0 0-1.5-1.5H3.75A1.5 1.5 0 0 0 2.25 6v12a1.5 1.5 0 0 0 1.5 1.5Z"
            />
          </svg>
          <span>Media Comparator</span>
        </RouterLink>

        <RouterLink
          to="/studio"
          class="flex items-center gap-2.5 px-3 py-2 rounded-xl text-xs font-medium transition-all"
          :class="
            isStudioActive
              ? 'bg-[#1e1e24] text-white shadow-sm'
              : 'text-neutral-400 hover:text-neutral-200 hover:bg-[#16161b]'
          "
        >
          <svg
            class="w-4 h-4 text-blue-400"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="m21 7.5-9-5.25L3 7.5m18 0-9 5.25m9-5.25v9l-9 5.25M3 7.5l9 5.25M3 7.5v9l9 5.25m0-9v9"
            />
          </svg>
          <span>3D World Studio</span>
        </RouterLink>
      </div>

      <!-- Recent Scans / Directories (Unsloth Project Style) -->
      <div class="px-3 pt-3 flex flex-col gap-1 border-t border-[#1c1c22]">
        <div class="flex items-center justify-between px-2 py-1">
          <span class="text-[10px] font-semibold uppercase tracking-wider text-neutral-500"
            >Recent Folders</span
          >
          <span class="text-[10px] font-mono text-neutral-600">{{
            mediaStore.recentDirectories.length
          }}</span>
        </div>

        <div
          v-if="mediaStore.recentDirectories.length === 0"
          class="px-2 py-3 text-[11px] text-neutral-600 italic"
        >
          No directories scanned yet.
        </div>

        <div v-else class="flex flex-col gap-0.5 max-h-48 overflow-y-auto">
          <button
            v-for="dir in mediaStore.recentDirectories"
            :key="dir"
            @click="openRecentFolder(dir)"
            type="button"
            class="flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-left text-xs font-mono transition-colors group"
            :class="
              mediaStore.directoryPath === dir
                ? 'bg-[#1a1a20] text-emerald-400'
                : 'text-neutral-400 hover:text-neutral-200 hover:bg-[#141418]'
            "
            :title="dir"
          >
            <svg
              class="w-3.5 h-3.5 shrink-0 opacity-60 group-hover:opacity-100"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              viewBox="0 0 24 24"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
              />
            </svg>
            <span class="truncate text-[11px]">{{ truncatePath(dir) }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Bottom System & Runtime Status -->
    <div class="p-3 border-t border-[#1c1c22] bg-[#0c0c10] flex flex-col gap-2">
      <div class="flex items-center justify-between px-1">
        <div class="flex items-center gap-2">
          <span
            class="w-2 h-2 rounded-full"
            :class="mediaStore.isVisionCapable ? 'bg-emerald-500' : 'bg-amber-400'"
          />
          <span class="text-[11px] font-mono text-neutral-400">Endpoint Ready</span>
        </div>
        <span class="text-[10px] font-mono text-neutral-500">v1/chat</span>
      </div>

      <!-- User Profile / App Info Row -->
      <div
        class="flex items-center justify-between p-2 rounded-xl bg-[#141418] border border-[#222228]"
      >
        <div class="flex items-center gap-2">
          <div
            class="w-6 h-6 rounded-lg bg-emerald-900/60 text-emerald-400 flex items-center justify-center font-bold text-xs"
          >
            U
          </div>
          <div class="flex flex-col">
            <span class="text-xs font-medium text-neutral-200">Local Studio</span>
            <span class="text-[9px] text-neutral-500 font-mono">localhost:3423</span>
          </div>
        </div>
        <button type="button" class="text-neutral-500 hover:text-neutral-300 p-1">
          <svg
            class="w-3.5 h-3.5"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 0 1 1.37.49l1.296 2.247a1.125 1.125 0 0 1-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 0 1 0 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 0 1-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 0 1-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 0 1-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 0 1-1.369-.49l-1.297-2.247a1.125 1.125 0 0 1 .26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 0 1 0-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 0 1-.26-1.43l1.297-2.247a1.125 1.125 0 0 1 1.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281Z"
            />
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"
            />
          </svg>
        </button>
      </div>
    </div>
  </aside>
</template>