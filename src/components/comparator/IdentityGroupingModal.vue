<script setup lang="ts">
import { computed } from 'vue'
import { useMediaStore } from '@/stores/mediaStore'
import { useAutoPipeline } from '@/composables/useAutoPipeline'
import type { MediaItem } from '@/types/media'

const mediaStore = useMediaStore()
const { stageAllClusterActions } = useAutoPipeline()

const totalFilesToGroup = computed(() => {
  return mediaStore.identityClusters.reduce((sum, c) => sum + c.items.length, 0)
})

function isItemExemplar(
  cluster: (typeof mediaStore.identityClusters)[number],
  itemId: string,
): boolean {
  return cluster.exemplars.some((ex) => ex.id === itemId)
}

function moveItemToCluster(sourceClusterId: string, item: MediaItem, targetClusterId: string) {
  if (sourceClusterId === targetClusterId) return

  mediaStore.removeItemFromCluster(sourceClusterId, item.id)

  if (targetClusterId === 'new') {
    const nextIdx = mediaStore.identityClusters.length + 1
    const defaultName = `person-${nextIdx}`
    mediaStore.identityClusters.push({
      id: `cluster-${Date.now()}`,
      suggestedName: defaultName,
      customName: defaultName,
      primaryItem: item,
      exemplars: [item],
      items: [item],
      confidenceAverage: 1.0,
    })
  } else {
    const target = mediaStore.identityClusters.find((c) => c.id === targetClusterId)
    if (target) {
      target.items.push(item)
    }
  }
}
</script>

<template>
  <div
    v-if="mediaStore.isIdentityReviewOpen"
    class="fixed inset-0 z-50 bg-black/85 backdrop-blur-sm flex items-center justify-center p-6 select-none"
  >
    <div
      class="w-full max-w-5xl bg-[#121217] border border-[#22222a] rounded-3xl shadow-2xl flex flex-col overflow-hidden max-h-[88vh]"
    >
      <div
        class="px-6 py-4 border-b border-[#202028] flex items-center justify-between bg-[#15151c]"
      >
        <div>
          <h3 class="text-sm font-semibold text-neutral-100 flex items-center gap-2">
            <span>👤 Multi-Exemplar Identity Clustering</span>
            <span
              class="px-2 py-0.5 rounded text-[10px] font-mono bg-emerald-950/80 text-emerald-400 border border-emerald-800"
            >
              {{ mediaStore.identityClusters.length }} Identities Discovered
            </span>
          </h3>
          <p class="text-xs text-neutral-400 mt-0.5">
            Groups formed using quality-ranked exemplar arbitration. You can rename folders or
            reassign items.
          </p>
        </div>
        <button
          @click="mediaStore.isIdentityReviewOpen = false"
          class="text-neutral-400 hover:text-neutral-200 text-xl leading-none"
        >
          &times;
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-6 flex flex-col gap-5">
        <div
          v-if="mediaStore.identityClusters.length === 0"
          class="p-8 border border-dashed border-[#262632] rounded-2xl text-center text-xs text-neutral-500"
        >
          No identity clusters generated.
        </div>

        <div
          v-for="cluster in mediaStore.identityClusters"
          :key="cluster.id"
          class="p-4 bg-[#16161e] border border-[#262632] rounded-2xl flex flex-col gap-3 shadow-md"
        >
          <div class="flex items-center justify-between gap-4">
            <div class="flex items-center gap-2.5 flex-1 max-w-md">
              <span class="text-xs font-semibold text-neutral-400 shrink-0">Person Name:</span>
              <input
                v-model="cluster.customName"
                type="text"
                placeholder="Identity name..."
                class="flex-1 bg-[#1c1c26] border border-[#2f2f3d] focus:border-emerald-500 rounded-xl px-3 py-1.5 text-xs text-neutral-100 font-medium outline-none transition-colors"
              />
            </div>

            <div class="flex items-center gap-3">
              <span class="text-[11px] font-mono text-neutral-400">
                Folder:
                <span class="text-emerald-400">
                  _identity_{{ cluster.customName.trim().replace(/[^a-zA-Z0-9_-]/g, '_') }}
                </span>
              </span>
              <button
                type="button"
                @click="mediaStore.removeCluster(cluster.id)"
                class="text-neutral-500 hover:text-red-400 text-xs px-2 py-1 transition-colors"
                title="Discard this group"
              >
                Discard Group
              </button>
            </div>
          </div>

          <!-- Thumbnails Grid -->
          <div class="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-6 gap-3 pt-1">
            <div
              v-for="item in cluster.items"
              :key="item.id"
              class="group relative aspect-square bg-[#0c0c10] border border-[#292936] rounded-xl overflow-hidden flex flex-col"
            >
              <img
                :src="item.thumbnailUrl"
                loading="lazy"
                decoding="async"
                class="w-full h-full object-cover"
              />

              <div
                v-if="item.mediaType === 'video'"
                class="absolute bottom-1 right-1 px-1 py-0.5 rounded text-[8px] font-mono bg-black/80 text-neutral-300 flex items-center gap-0.5 z-10"
              >
                VID
              </div>

              <!-- Primary Seed Badge -->
              <div
                v-if="item.id === cluster.primaryItem.id"
                class="absolute top-1 left-1 px-1.5 py-0.5 rounded text-[8px] font-mono bg-emerald-950/90 text-emerald-300 border border-emerald-700 z-10"
              >
                SEED
              </div>
              <!-- Secondary Exemplar Badge -->
              <div
                v-else-if="isItemExemplar(cluster, item.id)"
                class="absolute top-1 left-1 px-1.5 py-0.5 rounded text-[8px] font-mono bg-blue-950/90 text-blue-300 border border-blue-700 z-10"
              >
                EXEMPLAR
              </div>

              <!-- Reassign Dropdown Overlay -->
              <div
                class="absolute inset-x-0 bottom-0 bg-black/85 backdrop-blur-md p-1.5 flex flex-col gap-1 transition-transform translate-y-full group-hover:translate-y-0 duration-200 z-20"
              >
                <span class="text-[9px] text-neutral-400 truncate">{{ item.name }}</span>
                <select
                  @change="
                    (e) =>
                      moveItemToCluster(cluster.id, item, (e.target as HTMLSelectElement).value)
                  "
                  class="bg-[#1c1c24] border border-[#30303e] rounded px-1.5 py-0.5 text-[9px] text-neutral-200 outline-none cursor-pointer"
                >
                  <option value="" disabled selected>Move to...</option>
                  <option
                    v-for="c in mediaStore.identityClusters"
                    :key="c.id"
                    :value="c.id"
                    :disabled="c.id === cluster.id"
                  >
                    {{ c.customName }}
                  </option>
                  <option value="new">+ Split to New Group</option>
                </select>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div
        class="px-6 py-4 border-t border-[#202028] bg-[#14141a] flex items-center justify-between"
      >
        <button
          type="button"
          @click="mediaStore.isIdentityReviewOpen = false"
          class="px-4 py-2 text-xs font-medium text-neutral-400 hover:text-neutral-200 transition-colors"
        >
          Cancel
        </button>

        <div class="flex items-center gap-3">
          <span class="text-xs text-neutral-400">
            Total files to group: <strong>{{ totalFilesToGroup }}</strong>
          </span>
          <button
            type="button"
            :disabled="totalFilesToGroup === 0"
            @click="stageAllClusterActions"
            class="px-5 py-2 bg-emerald-600 hover:bg-emerald-500 active:bg-emerald-700 disabled:opacity-40 text-white text-xs font-semibold rounded-xl shadow-lg transition-all flex items-center gap-1.5"
          >
            <span>Stage Folders for Review</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>