<script setup lang="ts">
import { computed } from 'vue'
import { taskModelIds, visualOperationModels, type VisualOperation } from '@/config/playground'
import type {
  AnalysisGroup,
  ModelCapabilities,
  Settings,
  VisualAnalysisModels,
} from '@/types/protocol'
import HubGroup from './HubGroup.vue'
import HubToggle from './HubToggle.vue'
import HubRow from './HubRow.vue'
import RuneChoice from './RuneChoice.vue'
const props = withDefaults(defineProps<{ draft: Settings; models?: ModelCapabilities[] }>(), {
  models: () => [],
})
const choices: { key: keyof VisualAnalysisModels; label: string; operation?: VisualOperation }[] = [
  { key: 'planner', label: 'Planning model' },
  { key: 'entities', label: 'Detection model', operation: 'detect' },
  { key: 'instances', label: 'Instance segmentation model', operation: 'instance-segment' },
  { key: 'oriented', label: 'Oriented detection model', operation: 'oriented-detect' },
  { key: 'regions', label: 'Segmentation model', operation: 'segment' },
  { key: 'structure', label: 'Pose model', operation: 'pose' },
  { key: 'depth', label: 'Depth model' },
]
const selected = computed(
  () =>
    props.draft.visualAnalysisModels ?? {
      planner: '',
      entities: '',
      regions: '',
      structure: '',
      depth: '',
    },
)
function options(choice: (typeof choices)[number]) {
  const allowed = props.draft.connections?.find(
    (connection) => connection.id === props.draft.activeConnectionId,
  )?.selectedModels
  const models = allowed ? props.models.filter((model) => allowed.includes(model.id)) : props.models
  const ids =
    choice.key === 'planner'
      ? models
          .filter((model) => model.imageIn === 'yes' && model.toolCalls === 'yes')
          .map((model) => model.id)
      : choice.operation
        ? visualOperationModels(models, choice.operation)
        : taskModelIds(models, 'image-to-depth')
  return [
    { value: '', label: choice.key === 'planner' ? 'Select a model' : 'Disabled' },
    ...(selected.value[choice.key] && !ids.includes(selected.value[choice.key]!)
      ? [
          {
            value: selected.value[choice.key]!,
            label: `${selected.value[choice.key]} · unavailable or incompatible`,
            disabled: true,
          },
        ]
      : []),
    ...ids.map((id) => ({ value: id, label: id })),
  ]
}
function setModel(key: keyof VisualAnalysisModels, value: string) {
  props.draft.visualAnalysisModels = { ...selected.value, [key]: value }
}
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
    title="Visual Analysis models"
    hint="The Playground Flow and screen overlay use these choices. Leave an Operation disabled to skip it."
  >
    <HubRow v-for="choice in choices" :key="choice.key" :label="choice.label">
      <RuneChoice
        :model-value="selected[choice.key] ?? ''"
        :label="choice.label"
        :options="options(choice)"
        menu
        @update:model-value="setModel(choice.key, $event)"
      />
    </HubRow>
  </HubGroup>
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