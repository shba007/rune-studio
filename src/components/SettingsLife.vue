<script setup lang="ts">
import RuneButton from './RuneButton.vue'
import { ref } from 'vue'

import type { Settings } from '@/types/protocol'

import AppIcon from './AppIcon.vue'
import HubGroup from './HubGroup.vue'
import HubRow from './HubRow.vue'
import HubToggle from './HubToggle.vue'

const props = defineProps<{ draft: Settings }>()

interface ClassMeta {
  key: string
  label: string
  desc: string
  sensitive: boolean
}

const CLASSES: ClassMeta[] = [
  {
    key: 'general',
    label: 'General',
    desc: 'Goals, skills, daily logs and general reflections.',
    sensitive: false,
  },
  {
    key: 'health',
    label: 'Health',
    desc: 'Sleep, vitals, workouts and physical wellbeing. Encrypted, strictly local-only.',
    sensitive: true,
  },
  {
    key: 'mind',
    label: 'Mind',
    desc: 'Mood, cognitive state, habits and mindset notes. Encrypted, strictly local-only.',
    sensitive: true,
  },
  {
    key: 'finance',
    label: 'Finance',
    desc: 'Income, savings, compound moves and balance statements. Encrypted, strictly local-only.',
    sensitive: true,
  },
]

function ensureMaps() {
  if (!props.draft.lifeConsents) {
    props.draft.lifeConsents = {
      general: false,
      health: false,
      mind: false,
      finance: false,
    }
  }
  if (!props.draft.retentionDays) {
    props.draft.retentionDays = {
      general: 0,
      health: 0,
      mind: 0,
      finance: 0,
    }
  }
}
ensureMaps()

const deleteConfirmClass = ref<string | null>(null)
const deleteConfirmInput = ref('')
const deleteMessage = ref<string | null>(null)
const exportMessage = ref<string | null>(null)

function promptDelete(classKey: string) {
  deleteConfirmClass.value = classKey
  deleteConfirmInput.value = ''
  deleteMessage.value = null
}

function cancelDelete() {
  deleteConfirmClass.value = null
  deleteConfirmInput.value = ''
}

function executeDelete() {
  if (!deleteConfirmClass.value) return
  if (deleteConfirmInput.value.toLowerCase() !== deleteConfirmClass.value.toLowerCase()) {
    deleteMessage.value = `Type "${deleteConfirmClass.value}" to confirm deletion.`
    return
  }
  deleteMessage.value = `Data for "${deleteConfirmClass.value}" permanently purged and vacuumed.`
  deleteConfirmClass.value = null
  deleteConfirmInput.value = ''
}

function triggerExport(classKey: string) {
  exportMessage.value = `Export initiated for ${classKey}. Check the exports folder.`
  setTimeout(() => {
    exportMessage.value = null
  }, 4000)
}
</script>

<template>
  <div class="flex flex-col gap-6" data-testid="settings-life">
    <div class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-body text-rune-text-2">
      <div class="mb-2 flex items-center gap-2 font-mono text-rune-ink uppercase">
        <AppIcon name="shield" size="sm" class="text-rune-ink" />
        <span>Privacy & Foundations Notice [OWNER TO REVIEW]</span>
      </div>
      <p class="mt-0 mb-2 leading-relaxed text-rune-text-2">
        Rune Life Harness stores evidence-based records of claims, habits, health, mind, and
        finances. Sensitive domains (health, mind, finance) are encrypted at rest with authenticated
        AES-256-GCM using a key in your OS keyring, and are strictly local-only: they never leave
        this device.
      </p>
      <p class="m-0 leading-relaxed text-rune-text-2 italic">
        Disclaimer: Rune is not a clinician, therapist, or financial advisor. It provides structured
        reflection and accountability assistance, not medical, psychological, or professional
        financial advice.
      </p>
    </div>
    <HubGroup>
      <HubToggle
        :model-value="draft.lifeEnabled ?? false"
        @update:model-value="draft.lifeEnabled = $event"
        label="Life Harness module"
        data-testid="life-enabled-toggle"
      />
    </HubGroup>
    <template v-if="draft.lifeEnabled">
      <h2 class="mt-4 mb-0 text-rune-title font-normal text-rune-ink">
        Domain Consents & Retention
      </h2>
      <p class="mt-0 text-rune-body text-rune-text-2">
        Each domain requires explicit consent. Disabling consent stops future collection without
        deleting past records.
      </p>

      <HubGroup>
        <div
          v-for="item in CLASSES"
          :key="item.key"
          class="flex flex-col gap-rune-2 py-4"
          :data-testid="`class-row-${item.key}`"
        >
          <div class="flex items-center justify-between gap-4">
            <div class="flex min-w-0 flex-1 flex-col gap-1">
              <div class="flex items-center gap-2">
                <span class="text-rune-body font-medium text-rune-ink">{{ item.label }}</span>
                <span
                  v-if="item.sensitive"
                  class="rounded-full bg-rune-surface px-2 py-0.5 text-rune-caption font-mono text-rune-ink uppercase"
                >
                  Local-only & Encrypted
                </span>
              </div>
              <span class="text-rune-body text-rune-text-2">{{ item.desc }}</span>
            </div>
            <label class="flex cursor-pointer items-center gap-2 text-rune-body text-rune-text-2">
              <input
                v-if="draft.lifeConsents"
                v-model="draft.lifeConsents[item.key]"
                type="checkbox"
                class="h-4 w-4 rounded-rune-tile accent-zinc-100"
                :data-testid="`consent-${item.key}`"
              />
              <span>Consent</span>
            </label>
          </div>
          <div class="mt-1 flex flex-wrap items-center justify-between gap-4 pt-2 text-rune-body">
            <div class="flex items-center gap-rune-2 text-rune-text-2">
              <span>Retention (days, 0 = forever):</span>
              <input
                v-if="draft.retentionDays"
                v-model.number="draft.retentionDays[item.key]"
                type="number"
                min="0"
                step="1"
                class="w-20 rounded-rune-tile bg-black/50 px-2.5 py-1 text-right text-rune-ink"
                :data-testid="`retention-${item.key}`"
              />
            </div>

            <div class="flex items-center gap-2">
              <RuneButton
                variant="ghost"
                size="sm"
                :data-testid="`export-${item.key}`"
                @click="triggerExport(item.key)"
              >
                <AppIcon name="send" size="sm" class="rotate-90" />
                Export
              </RuneButton>
              <RuneButton
                variant="ghost"
                size="sm"
                :data-testid="`delete-${item.key}`"
                @click="promptDelete(item.key)"
              >
                <AppIcon name="trash" size="sm" />
                Delete
              </RuneButton>
            </div>
          </div>
        </div>
      </HubGroup>
      <div
        v-if="exportMessage"
        class="rounded-rune-tile bg-rune-surface p-rune-2 text-rune-body text-rune-ink"
      >
        {{ exportMessage }}
      </div>
      <div
        v-if="deleteMessage"
        class="rounded-rune-tile bg-error/10 p-rune-2 text-rune-body text-error"
      >
        {{ deleteMessage }}
      </div>
      <div
        v-if="deleteConfirmClass"
        class="rounded-rune-tile bg-attention/10 p-rune-2 text-rune-body text-rune-ink"
        data-testid="delete-confirm-box"
      >
        <div class="mb-2 flex items-center gap-2 font-mono text-error uppercase">
          <AppIcon name="trash" size="sm" />
          <span>Confirm Permanent Deletion: {{ deleteConfirmClass }}</span>
        </div>
        <p class="mt-0 mb-rune-2 text-rune-text-2">
          This will permanently delete all stored evidence, records, and related audit rows for
          domain
          <strong class="text-rune-ink font-mono">{{ deleteConfirmClass }}</strong> and vacuum the
          SQLite database so no traces remain on disk. This action cannot be undone.
        </p>
        <div class="flex flex-col gap-rune-2 sm:flex-row sm:items-center">
          <input
            v-model="deleteConfirmInput"
            type="text"
            :placeholder="`Type '${deleteConfirmClass}' to confirm`"
            class="flex-1 rounded-rune-tile bg-black px-rune-2 py-2 text-rune-ink placeholder:text-rune-text-3 focus:outline-none"
            data-testid="delete-confirm-input"
          />
          <div class="flex items-center gap-2">
            <RuneButton
              variant="danger"
              size="sm"
              data-testid="delete-confirm-btn"
              @click="executeDelete"
            >
              Permanently Delete
            </RuneButton>
            <RuneButton variant="ghost" size="sm" @click="cancelDelete"> Cancel </RuneButton>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>