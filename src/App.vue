<script setup lang="ts">
import { RouterView } from 'vue-router'
import AppSidebar from '@/components/shell/AppSidebar.vue'
import AppHeader from '@/components/shell/AppHeader.vue'
</script>

<template>
  <main
    class="flex w-screen h-screen overflow-hidden bg-[#0c0c0e] text-neutral-100 font-sans select-none"
  >
    <!-- Unsloth Left Navigation Sidebar -->
    <AppSidebar />

    <!-- Main Workspace Container -->
    <div class="flex-1 flex flex-col h-full overflow-hidden">
      <AppHeader />
      <div class="flex-1 overflow-hidden relative">
        <RouterView />
      </div>
    </div>
  </main>
</template>