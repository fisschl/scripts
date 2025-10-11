<script setup lang="ts">
import { usePreferredDark } from '@vueuse/core'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import AppLayout from '@/components/AppLayout.vue'

/**
 * 获取用户系统深色模式偏好
 */
const isDark = usePreferredDark()

/**
 * 监听深色模式变化，自动切换应用主题
 */
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
