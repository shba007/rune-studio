<script setup lang="ts">
import type { AnalysisGroup, Settings } from '@/types/protocol'
import HubGroup from './HubGroup.vue'
const props = defineProps<{ draft: Settings }>()
const groups: { key: AnalysisGroup; label: string; hint: string }[] = [
  { key: 'entities', label: 'Entities', hint: 'Objects, people, text and interface elements.' },
  {
    key: 'regions',
    label: 'Regions & Context',
    hint: 'Scene regions, image categories and context.',
  },
  { key: 'structure', label: 'Structure & Landmarks', hint: 'Pose, joints and visible landmarks.' },
  { key: 'depth', label: 'Depth', hint: 'Relative distance and spatial relationships.' },
]
function setVisible(key: AnalysisGroup, event: Event) {
  props.draft.visualAnalysis = {
    entities: true,
    regions: true,
    structure: true,
    depth: true,
    ...props.draft.visualAnalysis,
    [key]: (event.target as HTMLInputElement).checked,
  }
}
</script>
<template>
  <HubGroup title="Visual Analysis">
    <p class="m-0 text-rune-body text-rune-text-2">
      The model chooses observations relevant to your question. Uncheck a group to hide its screen
      highlights. Findings remain in the answer and saved task; these settings do not limit
      analysis.
    </p>
    <label
      v-for="group in groups"
      :key="group.key"
      class="flex cursor-pointer items-center gap-rune-2 py-2"
    >
      <input
        type="checkbox"
        :checked="draft.visualAnalysis?.[group.key] !== false"
        :aria-label="group.label"
        class="size-4 shrink-0 accent-rune-ink"
        @change="setVisible(group.key, $event)"
      />
      <span
        ><span class="block text-rune-body text-rune-ink">{{ group.label }}</span>
        <span class="text-rune-body text-rune-text-2">{{ group.hint }}</span></span
      >
    </label>
  </HubGroup>
</template>