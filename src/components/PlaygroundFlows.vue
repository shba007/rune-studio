<script setup lang="ts">
import { ref, watch, type Component } from 'vue'
import { FLOWS } from '@/config/flows'
import type { ModelCapabilities, VisualAnalysisModels } from '@/types/protocol'
import AppIcon from './AppIcon.vue'
import PlaygroundImageVoiceFlow from './PlaygroundImageVoiceFlow.vue'
import PlaygroundVisualAnalysisFlow from './PlaygroundVisualAnalysisFlow.vue'
import PlaygroundPortfolioFlow from './PlaygroundPortfolioFlow.vue'
import RuneNavItem from './RuneNavItem.vue'

const props = defineProps<{
  models: ModelCapabilities[]
  sidebarOpen: boolean
  selectedFlow?: string
  flowRequest?: number
  visualModels?: VisualAnalysisModels
  saveVisualModels?: (models: VisualAnalysisModels) => Promise<boolean>
}>()
const selected = ref(
  FLOWS.some((flow) => flow.id === props.selectedFlow) ? props.selectedFlow! : FLOWS[0]!.id,
)
watch(
  () => props.flowRequest,
  () => {
    const id = props.selectedFlow
    if (id && FLOWS.some((flow) => flow.id === id)) selected.value = id
  },
)
const flowComponents: Record<string, Component> = {
  'portfolio-studio': PlaygroundPortfolioFlow,
  'image-voice': PlaygroundImageVoiceFlow,
  'visual-analysis': PlaygroundVisualAnalysisFlow,
}
</script>

<template>
  <div class="flex min-h-0 flex-1" data-testid="flows-section">
    <nav
      v-show="sidebarOpen"
      class="glass flex w-64 shrink-0 flex-col gap-2 bg-rune-sunken px-4 pt-4"
      aria-label="Flows"
    >
      <h2 class="m-0 px-2 text-rune-heading text-rune-text-3 uppercase">Flows</h2>
      <RuneNavItem
        v-for="flow in FLOWS"
        :key="flow.id"
        :selected="selected === flow.id"
        :data-testid="`flow-${flow.id}`"
        @click="selected = flow.id"
      >
        <AppIcon :name="flow.icon" /><span class="min-w-0 flex-1 truncate">{{ flow.name }}</span>
      </RuneNavItem>
    </nav>
    <KeepAlive>
      <component
        :is="flowComponents[selected]"
        :key="selected"
        :models="models"
        :visual-models="visualModels"
        :save-visual-models="saveVisualModels"
      />
    </KeepAlive>
  </div>
</template>