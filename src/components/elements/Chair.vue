<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import type { Group } from 'three'
import { globalTools } from '@/services/toolRegistry';
import { normalizeModel } from '@/utils/normalizeModel';

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
      const deg = parseFloat(args.degrees) || 45
      dynamicRotation.value += (deg * Math.PI) / 180
      console.log(`[ChairItem] Swiveled chair by ${deg}°. Total: ${(dynamicRotation.value * 180 / Math.PI).toFixed(0)}°`)
    },
  })

  // 2. Load chair.glb and normalize with reusable utility
  const loader = new GLTFLoader()
  loader.load(
    '/models/chair.glb',
    (gltf) => {
      const model = gltf.scene

      // Normalizes width (X) to 1.0m, centers X/Z, snaps bottom to Y = 0
      const baseDimensions = normalizeModel(model)

      console.log('[ChairItem] Loaded and normalized chair.glb! Base dimensions:', {
        width: baseDimensions.x.toFixed(2), // 1.00m
        height: baseDimensions.y.toFixed(2),
        depth: baseDimensions.z.toFixed(2),
      })

      // Enable shadows on all child meshes
      model.traverse((node: any) => {
        if (node.isMesh) {
          node.castShadow = true
          node.receiveShadow = true
        }
      })

      loadedModel = model

      if (groupRef.value) {
        groupRef.value.add(model)
        console.log('[ChairItem] Attached chair.glb to scene!')
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
  <TresGroup ref="groupRef" :position="props.position"
    :rotation="[props.rotation[0], props.rotation[1] + dynamicRotation, props.rotation[2]]" :scale="props.scale" />
</template>