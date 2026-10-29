<script setup lang="ts">
import ModelIcon from './ModelIcon.vue'
import RuneChoice from './RuneChoice.vue'
import { computed, useId } from 'vue'
import type { ModelCapabilities } from '@/types/protocol'

const props = defineProps<{
  roleLabel: string
  roleKey: 'agent' | 'verifier' | 'analysis' | 'voice' | 'decision' | 'advisor'
  agentModel: string
  models: ModelCapabilities[]
  placeholder?: string
  selectedModels?: string[] | null
}>()

const modelValue = defineModel<string>({ required: true })

const effectiveModelId = computed(() => {
  const trimmed = modelValue.value.trim()
  if (trimmed) return trimmed
  return props.roleKey === 'agent' || props.roleKey === 'analysis' ? '' : props.agentModel.trim()
})

const currentCaps = computed(() => {
  if (!effectiveModelId.value) return null
  return props.models.find((m) => m.id === effectiveModelId.value) ?? null
})

const isCloud = computed(() => {
  return currentCaps.value?.locality === 'cloud'
})

const warning = computed<string | null>(() => {
  if (props.roleKey === 'analysis' && !modelValue.value.trim()) {
    return 'Choose an explicit Analysis model for Visual Analysis.'
  }
  const caps = currentCaps.value
  if (
    props.roleKey === 'analysis' &&
    (!caps || caps.imageIn !== 'yes' || caps.toolCalls !== 'yes')
  ) {
    return 'Choose a model that advertises image input and tool calling, then check the backend again.'
  }
  if (!caps) return null

  if (props.roleKey === 'voice' && caps.audioIn === 'no') {
    return 'This model does not support audio input. Voice will not work.'
  }
  if (props.roleKey === 'agent') {
    if (caps.imageIn === 'no')
      return 'This model lacks image input. Screen observation will not work.'
    if (caps.toolCalls === 'no') return 'This model lacks tool calling. Actions cannot be chosen.'
  }
  if (props.roleKey === 'verifier' && caps.imageIn === 'no') {
    return 'This model lacks image input. Visual analysis requires vision.'
  }
  return null
})

const inputId = useId()
const capabilities = computed(() => {
  const caps = currentCaps.value
  if (!caps) return []
  return [
    { label: 'See images', value: caps.imageIn },
    { label: 'Hear audio', value: caps.audioIn },
    { label: 'Speak aloud', value: caps.speechOut },
    { label: 'Use tools', value: caps.toolCalls },
    { label: 'Adjust thinking', value: caps.thinkingControl },
  ]
})
function statusLabel(status: string) {
  return status === 'yes' ? 'Supported' : status === 'no' ? 'Not supported' : 'Unknown'
}

const choices = computed(() => {
  const ids = props.selectedModels ?? []
  const stale = modelValue.value && !ids.includes(modelValue.value)
  return [
    {
      value: '',
      label:
        props.roleKey === 'agent' || props.roleKey === 'analysis'
          ? 'Choose a model'
          : 'Same as agent',
    },
    ...(stale
      ? [{ value: modelValue.value, label: `${modelValue.value} · not selected`, disabled: true }]
      : []),
    ...ids.map((id) => ({ value: id, label: id })),
  ]
})
</script>

<template>
  <div
    class="flex min-w-0 flex-col gap-2 bg-rune-surface px-rune-2 py-rune-2 first:rounded-t-rune-tile last:rounded-b-rune-tile"
  >
    <div class="flex flex-wrap items-center justify-between gap-x-8 gap-y-2">
      <div class="flex min-w-0 flex-1 flex-col gap-1">
        <label :for="inputId" class="text-rune-body text-rune-ink">{{ roleLabel }}</label>
        <p :id="`${inputId}-status`" class="m-0 break-words text-rune-caption text-rune-text-3">
          <template v-if="roleKey !== 'agent' && roleKey !== 'analysis' && !modelValue.trim()">
            Using agent model{{ effectiveModelId ? ` · ${effectiveModelId}` : '' }}.
          </template>
          <template v-if="currentCaps">
            {{
              isCloud
                ? 'Cloud model'
                : currentCaps.locality === 'local'
                  ? 'Local model'
                  : 'Model location unknown'
            }}
          </template>
          <template v-else>{{
            effectiveModelId
              ? 'Capabilities unknown. Check the connection or model name.'
              : 'Choose a model to see its capabilities.'
          }}</template>
        </p>
      </div>
      <div class="relative flex w-full min-w-0 items-center sm:w-72 sm:justify-end">
        <RuneChoice
          v-if="selectedModels"
          v-model="modelValue"
          :label="roleLabel"
          :options="choices"
          menu
        />
        <template v-else>
          <ModelIcon
            v-if="effectiveModelId"
            :model="effectiveModelId"
            class="pointer-events-none absolute top-3 left-3 text-rune-text-2"
          />
          <input
            :id="inputId"
            v-model="modelValue"
            type="text"
            list="discovered-models-list"
            :aria-describedby="`${inputId}-status`"
            :placeholder="
              placeholder ??
              (roleKey === 'analysis'
                ? 'Choose an Analysis model'
                : roleKey === 'agent'
                  ? 'Choose or enter a model'
                  : 'Same as agent')
            "
            spellcheck="false"
            autocomplete="off"
            :class="effectiveModelId ? 'pl-9' : ''"
            class="h-10 w-full min-w-0 rounded-rune-tile bg-rune-sunken px-3 text-rune-body text-rune-ink placeholder:text-rune-text-3"
          />
        </template>
      </div>
    </div>
    <details v-if="currentCaps" class="text-rune-caption text-rune-text-2">
      <summary class="w-fit cursor-pointer py-1 text-rune-caption text-rune-text-3">
        Model capabilities
      </summary>
      <dl class="m-0 mt-2 grid gap-x-8 gap-y-2 sm:grid-cols-2">
        <div
          v-for="cap in capabilities"
          :key="cap.label"
          class="flex items-center justify-between gap-3"
        >
          <dt class="flex items-center gap-2">
            <span
              aria-hidden="true"
              class="size-1.5 shrink-0 rounded-full"
              :class="
                cap.value === 'yes'
                  ? 'bg-rune-ink'
                  : cap.value === 'no'
                    ? 'bg-rune-raised'
                    : 'bg-rune-text-3'
              "
            />
            {{ cap.label }}
          </dt>
          <dd class="m-0 text-rune-text-2">{{ statusLabel(cap.value) }}</dd>
        </div>
      </dl>
    </details>
    <p
      v-if="warning"
      role="status"
      class="m-0 flex items-start gap-2 text-rune-caption text-attention"
    >
      <span aria-hidden="true" class="mt-1.5 size-1.5 shrink-0 rounded-full bg-attention" />
      {{ warning }}
    </p>
  </div>
</template>