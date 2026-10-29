<script setup lang="ts">
import type { AnalysisGroup, Settings } from '@/types/protocol'
import HubGroup from './HubGroup.vue'
import HubToggle from './HubToggle.vue'
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
function setVisible(key: AnalysisGroup, value: boolean) {
  props.draft.visualAnalysis = {
    entities: true,
    regions: true,
    structure: true,
    depth: true,
    ...props.draft.visualAnalysis,
    [key]: value,
  }
}
</script>
<template>
  <HubGroup
    title="Visual Analysis highlights"
    hint="The model chooses observations relevant to your question. Turn a group off to hide its screen highlights; findings stay in the answer and saved task."
  >
    <HubToggle
      v-for="group in groups"
      :key="group.key"
      :model-value="draft.visualAnalysis?.[group.key] !== false"
      :label="group.label"
      :hint="group.hint"
      @update:model-value="setVisible(group.key, $event)"
    />
  </HubGroup>
</template>