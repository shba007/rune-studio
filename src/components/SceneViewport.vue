<script setup lang="ts">
import type { Component } from 'vue'
import { OrbitControls } from '@tresjs/cientos'

// Elements
import EnvironmentLighting from '@/components/elements/EnvironmentLighting.vue'
import Surface from '@/components/elements/Surface.vue'
import Backdrop from '@/components/elements/Backdrop.vue'
import Table from '@/components/elements/Table.vue'
import Chair from '@/components/elements/Chair.vue'
import Printer from '@/components/elements/Printer.vue'
import PcMonitor from '@/components/elements/PcMonitor.vue'

import type { ScenePreset } from '@/types/scene.ts'

const props = defineProps<{
  scene: ScenePreset
}>()

const ENTITY_COMPONENTS: Record<string, Component> = {
  lighting: EnvironmentLighting,
  surface: Surface,
  backdrop: Backdrop,
  table: Table,
  chair: Chair,
  printer: Printer,
  monitor: PcMonitor,
}
</script>

<template>
  <TresGroup>
    <TresPerspectiveCamera make-default :position="props.scene.camera?.position || [14, 14, 14]"
      :look-at="props.scene.camera?.lookAt || [0, 1.5, 0]" :fov="props.scene.camera?.fov || 30" />
    <OrbitControls :enable-damping="true" />

    <component v-for="entity in props.scene.entities" :key="entity.id" :is="ENTITY_COMPONENTS[entity.type]"
      :position="entity.position || [0, 0, 0]" :rotation="entity.rotation || [0, 0, 0]" :scale="entity.scale || 1"
      v-bind="entity.props || {}" />
  </TresGroup>
</template>