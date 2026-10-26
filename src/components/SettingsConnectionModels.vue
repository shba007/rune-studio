<script setup lang="ts">
import { watch } from 'vue'
import type { Agent } from '@/composables/useAgent'
import type { Settings } from '@/types/protocol'
import BackendBadge from './BackendBadge.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
import ModelRoleRow from './ModelRoleRow.vue'
import AppIcon from './AppIcon.vue'
const props = defineProps<{ agent: Agent; draft: Settings }>()
const apiKey = defineModel<string>('apiKey', { default: '' })
defineEmits<{ 'commit-key': [] }>()
watch(
  () => props.draft.models.agent,
  (value) => {
    props.draft.model = value
  },
)
watch(
  () => props.draft.models.decision,
  (value) => {
    props.draft.decisionModel = value
  },
)
</script>
<template>
  <div class="flex flex-col gap-8">
    <HubGroup title="Connection">
      <HubRow label="Backend">
        <div class="max-w-96 min-w-0"><BackendBadge :status="agent.backend.value" /></div>
        <button
          type="button"
          class="grid size-8 cursor-pointer place-items-center rounded-full border-0 bg-transparent text-zinc-300 transition hover:bg-white/8 hover:text-white"
          aria-label="Check the backend again"
          @click="agent.refreshBackend()"
        >
          <AppIcon name="refresh" />
        </button>
      </HubRow>
      <HubInput v-model="draft.baseUrl" label="Base URL" />
      <HubInput
        v-model="apiKey"
        type="password"
        label="API key"
        :placeholder="agent.hasApiKey.value ? 'Stored. Type to replace' : 'Not set'"
        @commit="$emit('commit-key')"
      >
        <template #before>
          <button
            v-if="agent.hasApiKey.value"
            type="button"
            class="flex cursor-pointer items-center gap-1 border-0 bg-transparent text-label text-zinc-500 transition hover:text-[#d71921]"
            @click="agent.saveApiKey('')"
          >
            <AppIcon name="trash" size="sm" />
            Remove
          </button>
        </template>
      </HubInput>
    </HubGroup>

    <HubGroup title="Models">
      <p class="m-0 mb-2 text-label text-zinc-400">
        Choose a model for each job. Visual Analysis requires an explicit Analysis model with image
        input and tool calling. Other optional roles can use the agent model when empty. Open Model
        capabilities to check what each model supports.
      </p>
      <datalist id="discovered-models-list">
        <option v-for="m in agent.backend.value?.models ?? []" :key="m.id" :value="m.id" />
      </datalist>

      <ModelRoleRow
        v-model="draft.models.agent"
        role-label="Agent"
        role-key="agent"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
        placeholder="e.g. gemma-4-12b-it"
      />
      <ModelRoleRow
        v-model="draft.models.verifier"
        role-label="Verifier"
        role-key="verifier"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
      />
      <ModelRoleRow
        v-model="draft.models.analysis"
        role-label="Visual Analysis"
        role-key="analysis"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
      />
      <ModelRoleRow
        v-model="draft.models.voice"
        role-label="Voice"
        role-key="voice"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
      />
      <ModelRoleRow
        v-model="draft.models.decision"
        role-label="Decision"
        role-key="decision"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
      />
      <ModelRoleRow
        v-model="draft.models.advisor"
        role-label="Advisor"
        role-key="advisor"
        :agent-model="draft.models.agent"
        :models="agent.backend.value?.models ?? []"
      />

      <HubToggle
        v-model="draft.fastChecks"
        label="Fast checks"
        hint="A server that loads one model at a time swaps models for every fast check, which can add 10 seconds or more. Leave this off there."
      />
      <HubInput v-if="draft.fastChecks" v-model="draft.decisionUrl" label="Decision URL" />
      <HubToggle v-model="draft.showThinking" label="Show thinking" />
    </HubGroup>
  </div>
</template>