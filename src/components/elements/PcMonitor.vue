<script setup lang="ts">
import { ref, shallowRef, computed, onMounted, onUnmounted } from 'vue'
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
let screenMesh: any = null
let unregister: (() => void) | null = null

// Monitor reactive state
const isOn = ref(true)
const screenTheme = ref('code')
const screenColor = ref('#06b6d4') // Cyan code editor default

// Theme presets mapped to emissive glow colors
const THEME_COLORS: Record<string, string> = {
  code: '#06b6d4',      // Cyan glow
  matrix: '#22c55e',    // Hacker green
  terminal: '#22c55e',
  cyberpunk: '#ec4899', // Hot pink
  blue: '#3b82f6',
  white: '#f8fafc',
  amber: '#f59e0b',
}

const activeGlowColor = computed(() => {
  if (!isOn.value) return '#000000'
  return THEME_COLORS[screenTheme.value.toLowerCase()] || screenColor.value
})

function updateScreenMaterial() {
  if (!screenMesh || !screenMesh.material) return
  screenMesh.material.emissive.set(activeGlowColor.value)
  screenMesh.material.emissiveIntensity = isOn.value ? 0.8 : 0.0
  screenMesh.material.needsUpdate = true
}

onMounted(() => {
  // 1. Register Co-Located Tools for Needle 3
  unregister = globalTools.registerMany([
    // Tool 1: Monitor Power
    {
      name: 'set_monitor_power',
      aliases: ['toggle_monitor', 'turn_on_monitor', 'turn_off_monitor', 'power_monitor', 'set_pc_power'],
      description: 'Switch the PC monitor screen on or off',
      parameters: {
        type: 'object',
        properties: {
          state: { type: 'string', enum: ['on', 'off'] },
        },
        required: ['state'],
      },
      handler: async (args) => {
        isOn.value = String(args.state).toLowerCase() === 'on'
        updateScreenMaterial()
        console.log(`[PcMonitor] Power set to: ${isOn.value ? 'ON' : 'OFF'}`)
      },
    },

    // Tool 2: Screen Theme / Content
    {
      name: 'set_screen_display',
      aliases: ['set_screen_theme', 'display_on_monitor', 'change_wallpaper', 'open_editor', 'matrix_mode'],
      description: 'Change the display theme or color on the monitor (e.g. matrix, code, cyberpunk, red, blue)',
      parameters: {
        type: 'object',
        properties: {
          theme: {
            type: 'string',
            description: 'Theme name ("matrix", "code", "cyberpunk") or color name ("green", "purple", "#3b82f6")',
          },
        },
        required: ['theme'],
      },
      handler: async (args) => {
        const theme = String(args.theme || 'code').toLowerCase()
        screenTheme.value = theme
        screenColor.value = THEME_COLORS[theme] || theme
        isOn.value = true // Automatically power on when changing theme
        updateScreenMaterial()
        console.log(`[PcMonitor] Display changed to: ${theme}`)
      },
    },
  ])

  // 2. Load GLB via Three.js native loader (bypasses Suspense)
  const loader = new GLTFLoader()
  loader.load('/models/pc-monitor.glb', (gltf) => {
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
  <TresGroup ref="groupRef" :position="props.position" :rotation="props.rotation" :scale="props.scale">
    <!-- Screen Ambient Glow onto Desk surface -->
    <TresPointLight :position="[0, 0.45, 0.25]" :intensity="isOn ? 2.2 : 0.0" :color="activeGlowColor"
      :distance="2.2" />

    <!-- Standby Power LED on bottom bezel -->
    <TresMesh :position="[0.2, 0.04, 0.08]">
      <TresSphereGeometry :args="[0.015, 12, 12]" />
      <TresMeshStandardMaterial :color="isOn ? '#22c55e' : '#ef4444'" :emissive="isOn ? '#22c55e' : '#ef4444'"
        :emissive-intensity="0.8" />
    </TresMesh>
  </TresGroup>
</template>