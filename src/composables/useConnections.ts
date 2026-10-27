import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { onBeforeUnmount, onMounted, ref } from 'vue'
import type { BackendStatus } from '@/types/protocol'
import { problemText } from './useFlight'

export interface ConnectionView {
  id: string
  name: string
  baseUrl: string
  active: boolean
  hasApiKey: boolean
  selectedModels?: string[] | null
  additionalModels?: string[]
  outputLimit?: number | null
}
export interface ConnectionCatalog {
  revision: number
  activeId: string
  connections: ConnectionView[]
}
export interface ConnectionEdit {
  id: string | null
  name: string
  baseUrl: string
  key: string
  clearKey: boolean
}

export function useConnections() {
  const catalog = ref<ConnectionCatalog | null>(null)
  const statuses = ref<Record<string, BackendStatus>>({})
  const pending = ref(false)
  const testing = ref<string | null>(null)
  const error = ref('')
  let disposed = false
  let sequence = 0
  let off: (() => void) | undefined
  function apply(next: ConnectionCatalog) {
    if (catalog.value && catalog.value.revision !== next.revision) statuses.value = {}
    catalog.value = next
  }
  async function refresh() {
    const ticket = ++sequence
    try {
      const result = await invoke<ConnectionCatalog>('connections_state')
      if (!disposed && ticket === sequence) {
        apply(result)
        error.value = ''
      }
    } catch (err) {
      if (!disposed && ticket === sequence) error.value = problemText(err)
    }
  }
  onMounted(async () => {
    try {
      const stop = await listen('settings_changed', () => void refresh())
      if (disposed) {
        stop()
        return
      }
      off = stop
    } catch (err) {
      error.value = problemText(err)
    }
    await refresh()
  })
  onBeforeUnmount(() => {
    disposed = true
    sequence++
    off?.()
  })
  async function change(command: string, args: Record<string, unknown>) {
    if (pending.value || !catalog.value) return false
    pending.value = true
    error.value = ''
    try {
      const result = await invoke<ConnectionCatalog>(command, {
        revision: catalog.value.revision,
        ...args,
      })
      sequence++
      apply(result)
      return true
    } catch (err) {
      const failure = problemText(err)
      await refresh()
      error.value = failure
      return false
    } finally {
      pending.value = false
    }
  }
  async function check(id: string) {
    if (testing.value || !catalog.value) return
    const revision = catalog.value.revision
    testing.value = id
    error.value = ''
    try {
      const result = await invoke<BackendStatus>('connections_test', { id, revision })
      if (!disposed && catalog.value?.revision === revision)
        statuses.value = { ...statuses.value, [id]: result }
    } catch (err) {
      error.value = problemText(err)
    } finally {
      testing.value = null
    }
  }
  return {
    catalog,
    statuses,
    pending,
    testing,
    error,
    refresh,
    check,
    save: (edit: ConnectionEdit) =>
      change('connections_save', { ...edit, key: edit.key.trim() || null }),
    saveModels: (
      id: string,
      selectedModels: string[],
      additionalModels: string[],
      outputLimit: number | null,
    ) => change('connections_models_save', { id, selectedModels, additionalModels, outputLimit }),
    activate: (id: string) => change('connections_activate', { id }),
    remove: (id: string) => change('connections_delete', { id }),
  }
}