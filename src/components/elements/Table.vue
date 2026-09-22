<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { Group } from 'three'
import { normalizeModel } from '@/utils/normalizeModel';
import { globalTools } from '@/services/toolRegistry';

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

// Table state
const tableColor = ref<string | null>(null)

// Function to tint table materials
function applyTableColor(colorHex: string) {
  if (!loadedModel) return
  loadedModel.traverse((node: any) => {
    if (node.isMesh && node.material) {
      if (!node.material.userData.originalColor) {
        node.material.userData.originalColor = node.material.color.clone()
      }
      node.material.color.set(colorHex)
    }
  })
}

onMounted(() => {
  // 1. Register Co-Located Tools for Needle 3
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
        const resolvedColor = colorMap[args.color?.toLowerCase()] || args.color
        if (resolvedColor) {
          tableColor.value = resolvedColor
          applyTableColor(resolvedColor)
          console.log(`[TableItem] Table finish changed to: ${resolvedColor}`)
        }
      },
    },
  ])

  // 2. Load table.glb via Three.js native GLTFLoader
  const loader = new GLTFLoader()
  loader.load('/models/table.glb', (gltf) => {
    const model = gltf.scene
    const normalizedSize = normalizeModel(model)

    console.log('Model normalized to 1m width. Base height is:', normalizedSize.y)
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
  <TresGroup ref="groupRef" :position="props.position" :rotation="props.rotation" :scale="props.scale" />
</template>