<script setup lang="ts">
import { nextTick, useTemplateRef, watch } from 'vue'

import type { Agent } from '@/composables/useAgent'
import type { AgentState, Settings } from '@/types/protocol'

import BackendBadge from './BackendBadge.vue'
import DevInputLab from './DevInputLab.vue'
import HubGroup from './HubGroup.vue'
import HubInput from './HubInput.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
import ModelRoleRow from './ModelRoleRow.vue'
import AppIcon from './AppIcon.vue'
import SettingsReview from './SettingsReview.vue'

const props = defineProps<{ agent: Agent; draft: Settings }>()

if (!props.draft.models) {
  props.draft.models = {
    agent: props.draft.model || 'gemma-4-12b-it',
    verifier: '',
    analysis: '',
    voice: '',
    decision: props.draft.decisionModel || 'liquid-d1-3b',
    advisor: '',
  }
}

watch(
  () => props.draft.models.agent,
  (val) => {
    props.draft.model = val
  },
)

watch(
  () => props.draft.models.decision,
  (val) => {
    props.draft.decisionModel = val
  },
)

const apiKey = defineModel<string>('apiKey', { required: true })
defineEmits<{ 'commit-key': [] }>()

const isDev = import.meta.env.DEV
const DEBUG_STATES: AgentState[] = [
  'idle',
  'listening',
  'planning',
  'acting',
  'awaiting_confirm',
  'done',
  'failed',
]

const developer = useTemplateRef<HTMLDetailsElement>('developer')
async function revealDeveloper() {
  if (!developer.value?.open) return
  await nextTick()
  developer.value.scrollIntoView({ block: 'end', behavior: 'smooth' })
}
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
        Choose a model for each job. Leave a field empty to use the agent model. Open Model
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
        role-label="Screen Analysis"
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

    <HubGroup title="Reflection">
      <HubToggle
        v-model="draft.reflectionEnabled"
        label="Self reflect"
        :hint="`After ${draft.idleMinutes} idle minutes Rune asks first; no answer in 30 seconds starts it. Right now it only rehearses: it does not read anything yet.`"
      />
    </HubGroup>

    <HubGroup title="Recording">
      <SettingsReview :draft="draft" />
      <HubToggle
        v-model="draft.demoMode"
        label="Show Rune in screen recordings"
        hint="OBS, Game Bar and the Snipping Tool can capture Rune's windows. Rune hides itself from its own model either way."
      />
    </HubGroup>

    <HubGroup title="Safety">
      <HubToggle v-model="draft.askEveryAction" label="Ask before every action" />
    </HubGroup>

    <HubGroup title="Developer">
      <HubToggle
        v-model="draft.developerMode"
        label="Developer mode"
        hint="Rune Preview shows the raw record, prompts and model calls of each step."
      />
    </HubGroup>

    <details
      v-if="isDev"
      ref="developer"
      class="rounded-2xl bg-white/5 px-5 py-3"
      @toggle="revealDeveloper"
    >
      <summary class="cursor-pointer text-caption text-zinc-500 uppercase">Debug</summary>
      <div class="flex flex-wrap gap-2 pt-3">
        <button
          v-for="s in DEBUG_STATES"
          :key="s"
          type="button"
          class="h-8 cursor-pointer rounded-full border-0 bg-white/8 px-3 text-label text-zinc-300 transition hover:bg-white/15"
          @click="agent.debugSetState(s)"
        >
          {{ s }}
        </button>
      </div>
      <DevInputLab :agent="agent" class="mt-3" />
    </details>
  </div>
</template>