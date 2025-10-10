<script setup lang="ts">
import { usePreferredDark } from '@vueuse/core'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
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
  <ElConfigProvider :locale="zhCn">
    <AppLayout />
  </ElConfigProvider>
</template>

<style>
#app {
  height: 100vh;
  overflow: hidden;
  display: flex;
}

html {
  font-family: var(--font-sans);

  body {
    font-family: inherit;
  }
}

html,
body {
  background-color: transparent;
}
</style>
