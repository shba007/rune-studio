<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { Group, Mesh, type Object3D } from 'three'
import { normalizeModel } from '@/utils/normalizeModel'
import { globalTools } from '@/services/toolRegistry'

const props = withDefaults(
  defineProps<{
    position?: [number, number, number]
    rotation?: [number, number, number]
    scale?: [number, number, number] | number
  }>(),
  {
    position: () => [0, 0, 0],
    rotation: () => [0, 0, 0],
    scale: 1,
  },
)

const groupRef = shallowRef<Group | null>(null)
let loadedModel: Group | null = null
let unregister: (() => void) | null = null

const tableColor = ref<string | null>(null)

function applyTableColor(colorHex: string) {
  if (!loadedModel) return
  loadedModel.traverse((node: Object3D) => {
    if (node instanceof Mesh && node.material) {
      const materials = Array.isArray(node.material) ? node.material : [node.material]
      for (const mat of materials) {
        if ('color' in mat && mat.color) {
          mat.color.set(colorHex)
        }
      }
    }
  })
}

onMounted(() => {
  unregister = globalTools.registerMany([
    {
      name: 'set_table_color',
      aliases: ['change_desk_color', 'paint_table', 'table_material'],
      description: 'Change the color or wood finish of the table/desk',
      parameters: {
        type: 'object',
        properties: {
          color: {
            type: 'string',
            description: 'Color name (e.g. walnut, white, black, brown, #451a03) or hex',
          },
        },
        required: ['color'],
      },
      handler: async (args) => {
        const colorMap: Record<string, string> = {
          walnut: '#3f2212',
          oak: '#b5835a',
          white: '#f8fafc',
          black: '#18181b',
          dark: '#27272a',
        }
        const inputColor = typeof args.color === 'string' ? args.color : ''
        const resolvedColor = colorMap[inputColor.toLowerCase()] || inputColor
        if (resolvedColor) {
          tableColor.value = resolvedColor
          applyTableColor(resolvedColor)
        }
      },
    },
  ])

  const loader = new GLTFLoader()
  loader.load('/models/table.glb', (gltf) => {
    const model = gltf.scene
    normalizeModel(model)
    loadedModel = model
    if (tableColor.value) {
      applyTableColor(tableColor.value)
    }
    groupRef.value?.add(model)
  })
})

onUnmounted(() => {
  unregister?.()
  if (groupRef.value && loadedModel) {
    groupRef.value.remove(loadedModel)
  }
})
</script>

<template>
  <TresGroup
    ref="groupRef"
    :position="props.position"
    :rotation="props.rotation"
    :scale="props.scale"
  />
</template>