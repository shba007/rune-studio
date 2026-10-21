<script setup lang="ts">
import { computed } from 'vue'
import type { ModelCapabilities } from '@/types/protocol'

const props = defineProps<{
  roleLabel: string
  roleKey: 'agent' | 'verifier' | 'analysis' | 'voice' | 'decision' | 'advisor'
  agentModel: string
  models: ModelCapabilities[]
  placeholder?: string
}>()

const modelValue = defineModel<string>({ required: true })

const effectiveModelId = computed(() => {
  const trimmed = modelValue.value.trim()
  if (trimmed) return trimmed
  return props.roleKey === 'agent' ? '' : props.agentModel.trim()
})

const currentCaps = computed(() => {
  if (!effectiveModelId.value) return null
  return props.models.find((m) => m.id === effectiveModelId.value) ?? null
})

const isCloud = computed(() => {
  return currentCaps.value?.locality === 'cloud'
})

const warning = computed<string | null>(() => {
  const caps = currentCaps.value
  if (!caps) return null

  if (props.roleKey === 'voice' && caps.audioIn === 'no') {
    return 'This model does not support audio input. Voice will not work.'
  }
  if (props.roleKey === 'agent') {
    if (caps.imageIn === 'no')
      return 'This model lacks image input. Screen observation will not work.'
    if (caps.toolCalls === 'no') return 'This model lacks tool calling. Actions cannot be chosen.'
  }
  if ((props.roleKey === 'verifier' || props.roleKey === 'analysis') && caps.imageIn === 'no') {
    return 'This model lacks image input. Visual analysis requires vision.'
  }
  return null
})

function dotClass(status: 'yes' | 'no' | 'unknown' | undefined) {
  if (status === 'yes') return 'text-emerald-400 bg-emerald-400/20 border-emerald-400/40'
  if (status === 'no') return 'text-zinc-600 bg-zinc-800/40 border-zinc-700/40 line-through'
  return 'text-amber-400/80 bg-amber-400/10 border-amber-400/30'
}

function dotSymbol(status: 'yes' | 'no' | 'unknown' | undefined) {
  if (status === 'yes') return '●'
  if (status === 'no') return '○'
  return '?'
}
</script>

<template>
  <div class="flex flex-col gap-1.5 py-2 border-b border-white/5 last:border-b-0">
    <div class="flex items-center justify-between gap-3">
      <div class="flex items-center gap-2 min-w-28">
        <span class="text-label text-zinc-300 font-medium">{{ roleLabel }}</span>
        <span
          v-if="isCloud"
          class="rounded px-1.5 py-0.5 text-[9px] font-bold tracking-wider uppercase bg-sky-500/20 text-sky-300 border border-sky-400/30"
        >
          CLOUD
        </span>
      </div>

      <div class="flex-1 max-w-72">
        <input
          v-model="modelValue"
          type="text"
          list="discovered-models-list"
          :placeholder="placeholder ?? (roleKey === 'agent' ? 'model id' : 'Same as agent')"
          class="w-full rounded border border-white/10 bg-white/5 px-2.5 py-1 text-body text-zinc-100 placeholder:text-zinc-600 focus:border-white/25 focus:outline-none"
        />
      </div>
    </div>

    <div v-if="currentCaps" class="flex items-center gap-3 text-[10px] text-zinc-400 pl-0.5">
      <span class="text-caption text-zinc-500 uppercase tracking-wider">Caps:</span>
      <span
        :class="['px-1 py-0.5 rounded border text-[9px]', dotClass(currentCaps.imageIn)]"
        title="Image Input"
      >
        {{ dotSymbol(currentCaps.imageIn) }} IMG
      </span>
      <span
        :class="['px-1 py-0.5 rounded border text-[9px]', dotClass(currentCaps.audioIn)]"
        title="Audio Input"
      >
        {{ dotSymbol(currentCaps.audioIn) }} AUD
      </span>
      <span
        :class="['px-1 py-0.5 rounded border text-[9px]', dotClass(currentCaps.speechOut)]"
        title="Speech Output"
      >
        {{ dotSymbol(currentCaps.speechOut) }} SPK
      </span>
      <span
        :class="['px-1 py-0.5 rounded border text-[9px]', dotClass(currentCaps.toolCalls)]"
        title="Tool Calls"
      >
        {{ dotSymbol(currentCaps.toolCalls) }} TOOL
      </span>
      <span
        :class="['px-1 py-0.5 rounded border text-[9px]', dotClass(currentCaps.thinkingControl)]"
        title="Thinking Control"
      >
        {{ dotSymbol(currentCaps.thinkingControl) }} THINK
      </span>
    </div>

    <div
      v-if="warning"
      class="text-caption text-amber-400 bg-amber-400/10 border border-amber-400/20 rounded px-2 py-1 mt-0.5"
    >
      ⚠ {{ warning }}
    </div>
  </div>
</template>