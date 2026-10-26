import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import type { DecisionAnswer, IntakeEntry } from '@/types/decisions'
import { problemText } from './useFlight'
export type DecisionModel = ReturnType<typeof useDecisions>

export function useDecisions() {
  const entries = ref<IntakeEntry[]>([])
  const error = ref('')
  const pending = ref(false)
  const deferred = ref(new Set<string>())
  let disposed = false
  let sequence = 0
  const off: (() => void)[] = []
  const busy = computed(() =>
    entries.value.some((e) =>
      ['classifying', 'clarifying', 'dispatching', 'pathway_planning'].includes(e.status),
    ),
  )
  const questions = computed(() => entries.value.filter((e) => e.question))
  const current = computed(() =>
    questions.value.find((e) => !deferred.value.has(`${e.id}:${e.revision}`)),
  )
  async function refresh() {
    const revision = ++sequence
    try {
      const result = await invoke<IntakeEntry[]>('decisions_state')
      if (!disposed && revision === sequence) {
        entries.value = Array.isArray(result) ? result : []
        error.value = ''
      }
    } catch (err) {
      if (!disposed && revision === sequence) error.value = problemText(err)
    }
  }
  onMounted(async () => {
    try {
      for (const name of ['decisions_changed', 'settings_changed']) {
        const stop = await listen(name, () => void refresh())
        if (disposed) {
          stop()
          return
        }
        off.push(stop)
      }
    } catch (err) {
      error.value = problemText(err)
    }
    await refresh()
  })
  onBeforeUnmount(() => {
    disposed = true
    off.forEach((stop) => stop())
  })
  async function answer(value: DecisionAnswer) {
    if (pending.value) return
    pending.value = true
    error.value = ''
    try {
      await invoke('decision_answer', { ...value })
    } catch (err) {
      error.value = problemText(err)
    } finally {
      pending.value = false
      await refresh()
    }
  }
  async function pathwayAction(entry: IntakeEntry, pathwayId?: string) {
    if (pending.value) return
    pending.value = true
    error.value = ''
    try {
      await invoke(pathwayId ? 'decision_select_pathway' : 'decision_pathways', {
        id: entry.id,
        revision: entry.revision,
        ...(pathwayId ? { pathwayId } : {}),
      })
    } catch (err) {
      error.value = problemText(err)
    } finally {
      pending.value = false
      await refresh()
    }
  }
  return {
    entries,
    questions,
    current,
    busy,
    pending,
    error,
    refresh,
    answer,
    comparePathways: (entry: IntakeEntry) => pathwayAction(entry),
    selectPathway: (entry: IntakeEntry, id: string) => pathwayAction(entry, id),
    refine: async (entry: IntakeEntry) => {
      if (pending.value) return
      pending.value = true
      try {
        await invoke('decision_refine', { id: entry.id, revision: entry.revision })
      } catch (err) {
        error.value = problemText(err)
      } finally {
        pending.value = false
        await refresh()
      }
    },
    defer: (id: string) => {
      const e = entries.value.find((e) => e.id === id)
      if (e) deferred.value.add(`${id}:${e.revision}`)
    },
    resume: (id: string) => {
      const e = entries.value.find((e) => e.id === id)
      if (e) deferred.value.delete(`${id}:${e.revision}`)
    },
  }
}