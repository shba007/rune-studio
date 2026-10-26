<script setup lang="ts">
import type { Settings } from '@/types/protocol'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'
const props = defineProps<{ draft: Settings }>()
props.draft.lifeConsents ??= { general: false, health: false, mind: false, finance: false }
props.draft.inputClassification ??= 'automatic'
</script>
<template>
  <HubGroup title="Input and goal context">
    <HubRow
      label="Input classification"
      hint="Automatic asks when the intended scope is unclear. Always confirm asks you to choose Goal, Task or Step every time."
    >
      <select
        v-model="draft.inputClassification"
        aria-label="Input classification"
        class="rounded-lg bg-zinc-900 px-3 py-2 text-body text-zinc-100"
      >
        <option value="automatic">Automatic</option>
        <option value="always_confirm">Always confirm</option>
      </select>
    </HubRow>
    <HubToggle
      :model-value="draft.lifeEnabled ?? false"
      @update:model-value="draft.lifeEnabled = $event"
      label="Save goal conversations"
      hint="Allow opted-in domains to be saved in the protected Life store and resumed after restart. New goal conversations use a local model."
    />
    <template v-if="draft.lifeEnabled">
      <HubToggle
        v-for="domain in ['general', 'health', 'mind', 'finance']"
        :key="domain"
        :model-value="draft.lifeConsents?.[domain] ?? false"
        @update:model-value="draft.lifeConsents![domain] = $event"
        :label="`Allow ${domain} context`"
        hint="Turning this off stops new storage and hides saved conversations; it does not delete existing records."
      />
    </template>
  </HubGroup>
</template>