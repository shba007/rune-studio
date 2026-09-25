<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { globalTools } from '@/services/toolRegistry'

const props = withDefaults(
  defineProps<{
    ambientIntensity?: number
    directionalIntensity?: number
    directionalPosition?: [number, number, number]
    color?: string
  }>(),
  {
    ambientIntensity: 0.9,
    directionalIntensity: 1.5,
    directionalPosition: () => [10, 18, 8],
    color: '#ffffff',
  },
)

const isLightOn = ref(true)
const dynamicIntensity = ref(props.ambientIntensity)
let unregister: (() => void) | null = null

onMounted(() => {
  unregister = globalTools.register({
    name: 'set_room_lighting',
    aliases: ['toggle_room_lights', 'set_ambient_light', 'room_lights'],
    description: 'Toggle or adjust ambient room ceiling lights',
    parameters: {
      type: 'object',
      properties: {
        state: { type: 'string', enum: ['on', 'off'] },
        intensity: { type: 'number', description: 'Intensity from 0.1 to 2.0' },
      },
    },
    handler: (args) => {
      if (args.state) {
        isLightOn.value = String(args.state).toLowerCase() === 'on'
      }
      if (args.intensity != null) {
        dynamicIntensity.value = Number(args.intensity)
      }
    },
  })
})

onUnmounted(() => {
  unregister?.()
})
</script>

<template>
  <TresGroup>
    <TresAmbientLight :color="props.color" :intensity="isLightOn ? dynamicIntensity : 0.1" />
    <TresDirectionalLight
      :color="props.color"
      :position="props.directionalPosition"
      :intensity="isLightOn ? props.directionalIntensity : 0.0"
      cast-shadow
    />
  </TresGroup>
</template>