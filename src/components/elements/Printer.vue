<script setup lang="ts">
import { ref, shallowRef, onMounted, onUnmounted } from 'vue'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { Group } from 'three'
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

const isPrinting = ref(false)
const isScanning = ref(false)
const paperOffset = ref(0)
let unregister: (() => void) | null = null
let loadedModel: Group | null = null

onMounted(() => {
  unregister = globalTools.registerMany([
    {
      name: 'print_document',
      aliases: ['print', 'printer_print', 'start_print'],
      description: 'Print a document on the printer',
      parameters: {
        type: 'object',
        properties: {
          document_name: { type: 'string' },
          copies: { type: 'number' },
        },
      },
      handler: async () => {
        isPrinting.value = true
        paperOffset.value = 0.35
        setTimeout(() => {
          isPrinting.value = false
        }, 3000)
      },
    },
    {
      name: 'scan_document',
      aliases: ['scan', 'printer_scan', 'start_scan'],
      description: 'Scan a document on the printer',
      parameters: {
        type: 'object',
        properties: {
          target: { type: 'string' },
        },
      },
      handler: async () => {
        isScanning.value = true
        setTimeout(() => {
          isScanning.value = false
        }, 2500)
      },
    },
  ])

  const loader = new GLTFLoader()
  loader.load('/models/printer.glb', (gltf) => {
    const model = gltf.scene
    normalizeModel(model)
    loadedModel = model
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
  >
    <TresPointLight
      :position="[0.3, 0.45, 0.25]"
      :intensity="isPrinting || isScanning ? 2.5 : 0.3"
      :color="isPrinting ? '#22c55e' : isScanning ? '#3b82f6' : '#64748b'"
      :distance="1.5"
    />

    <TresMesh :position="[0.3, 0.45, 0.25]">
      <TresSphereGeometry :args="[0.02, 16, 16]" />
      <TresMeshStandardMaterial
        :color="isPrinting ? '#22c55e' : isScanning ? '#3b82f6' : '#334155'"
        :emissive="isPrinting ? '#22c55e' : isScanning ? '#3b82f6' : '#000000'"
        :emissive-intensity="isPrinting || isScanning ? 1.0 : 0.0"
      />
    </TresMesh>

    <TresMesh
      v-if="paperOffset > 0"
      :position="[0, 0.08, 0.3]"
      :rotation="[-0.05, 0, 0]"
      cast-shadow
    >
      <TresPlaneGeometry :args="[0.35, 0.45]" />
      <TresMeshStandardMaterial color="#ffffff" :roughness="0.2" />
    </TresMesh>
  </TresGroup>
</template>