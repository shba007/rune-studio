import { computed, ref, shallowRef, watch, type Ref } from 'vue'

export function useSectionHistory<T>(current: Ref<T>) {
  const stack = shallowRef<T[]>([current.value])
  const index = ref(0)
  let travelling = false

  watch(current, (value) => {
    if (travelling) {
      travelling = false
      return
    }
    if (value === stack.value[index.value]) return
    stack.value = [...stack.value.slice(0, index.value + 1), value]
    index.value = stack.value.length - 1
  })

  const canBack = computed(() => index.value > 0)
  const canForward = computed(() => index.value < stack.value.length - 1)

  function go(delta: number) {
    const target = index.value + delta
    if (target < 0 || target >= stack.value.length) return
    index.value = target
    travelling = true
    current.value = stack.value[target]!
  }

  return { canBack, canForward, back: () => go(-1), forward: () => go(1) }
}