<script setup lang="ts">
import type { Settings } from '@/types/protocol'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
import RuneChoice from './RuneChoice.vue'
const props = defineProps<{ draft: Settings }>()
props.draft.lifeConsents ??= { general: false, health: false, mind: false, finance: false }
props.draft.inputClassification ??= 'automatic'

const CLASSIFICATION: { value: NonNullable<Settings['inputClassification']>; label: string }[] = [
  { value: 'automatic', label: 'Automatic' },
  { value: 'always_confirm', label: 'Always confirm' },
]
</script>
<template>
  <div class="flex flex-col gap-8">
    <HubGroup title="Input and goal context">
      <HubRow
        label="Input classification"
        hint="Choose when Rune asks whether input is a Goal, Task or Step."
        info="Automatic asks when the intended scope is unclear. Always confirm asks you to choose Goal, Task or Step every time."
      >
        <RuneChoice
          :model-value="draft.inputClassification ?? 'automatic'"
          @update:model-value="draft.inputClassification = $event"
          label="Input classification"
          :options="CLASSIFICATION"
        />
      </HubRow>
      <HubToggle
        :model-value="draft.lifeEnabled ?? false"
        label="Save goal conversations"
        hint="Save opted-in domains in the protected Life store and resume them after restart."
        info="Allow opted-in domains to be saved in the protected Life store and resumed after restart. New goal conversations use a local model."
        @update:model-value="draft.lifeEnabled = $event"
      />
    </HubGroup>
    <HubGroup
      v-if="draft.lifeEnabled"
      title="Context consent"
      hint="Turning a category off stops new storage and hides saved conversations; it does not delete existing records."
    >
      <HubToggle
        v-for="domain in ['general', 'health', 'mind', 'finance']"
        :key="domain"
        :model-value="draft.lifeConsents?.[domain] ?? false"
        :label="`Allow ${domain} context`"
        @update:model-value="draft.lifeConsents![domain] = $event"
      />
    </HubGroup>
  </div>
</template>