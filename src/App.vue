<script setup lang="ts">
import { usePreferredDark } from '@vueuse/core'
import { watchEffect } from 'vue'
import AppLayout from '@/components/AppLayout.vue'

const isDark = usePreferredDark()

watchEffect(() => {
  const { classList } = document.documentElement
  if (isDark.value)
    classList.add('dark')
  else classList.remove('dark')
})
</script>

<template>
  <AppLayout />
</template>

<style>
#app {
  height: 100vh;
  overflow: hidden;
  display: flex;
}

html {
  font-family: var(--font-sans);
  color-scheme: light dark;

  body {
    font-family: inherit;
  }
}
</style>
