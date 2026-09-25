<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { Group, Mesh, type Object3D } from 'three'
import { globalTools } from '@/services/toolRegistry'
import { normalizeModel } from '@/utils/normalizeModel'

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
const dynamicRotation = ref(0)
let loadedModel: Group | null = null
let unregister: (() => void) | null = null

onMounted(() => {
  unregister = globalTools.register({
    name: 'rotate_chair',
    aliases: ['rotate_item', 'turn_chair'],
    description: 'Rotate the chair by degrees (e.g. 45, 90, 180)',
    parameters: {
      type: 'object',
      properties: {
        degrees: {
          type: 'number',
          description: 'Rotation degrees',
        },
      },
    },
    handler: (args) => {
      const deg = parseFloat(String(args.degrees)) || 45
      dynamicRotation.value += (deg * Math.PI) / 180
    },
  })

  const loader = new GLTFLoader()
  loader.load(
    '/models/chair.glb',
    (gltf) => {
      const model = gltf.scene
      normalizeModel(model)

      model.traverse((node: Object3D) => {
        if (node instanceof Mesh) {
          node.castShadow = true
          node.receiveShadow = true
        }
      })

      loadedModel = model

      if (groupRef.value) {
        groupRef.value.add(model)
      }
    },
    undefined,
    (error) => {
      console.error('[ChairItem] Failed to load /models/chair.glb:', error)
    },
  )
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
    :rotation="[props.rotation[0], props.rotation[1] + dynamicRotation, props.rotation[2]]"
    :scale="props.scale"
  />
</template>