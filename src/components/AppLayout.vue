<script setup lang="ts">
import { Copy, GitBranch, Hash, RefreshCw } from 'lucide-vue-next'

/**
 * 获取当前路由信息
 */
const route = useRoute()

/**
 * 导航菜单项配置
 */
const examples = computed(() => {
  return [
    {
      name: '计算文件哈希值',
      path: '/file-hash',
      icon: markRaw(Hash),
      isActive: route.path.startsWith('/file-hash'),
    },
    {
      name: '文件复制',
      path: '/file-copy',
      icon: markRaw(Copy),
      isActive: route.path.startsWith('/file-copy'),
    },
    {
      name: 'Git 仓库克隆',
      path: '/repo-clone',
      icon: markRaw(GitBranch),
      isActive: route.path.startsWith('/repo-clone'),
    },
    {
      name: 'S3 同步',
      path: '/s3-sync',
      icon: markRaw(RefreshCw),
      isActive: route.path.startsWith('/s3-sync'),
    },
  ]
})

/**
 * 获取当前激活的菜单项
 */
const activeExample = computed(() => {
  return examples.value.find(example => example.isActive)
})
</script>

<template>
  <ElScrollbar
    :class="$style.aside"
    class="h-full shrink-0"
  >
    <ElMenu
      :default-active="activeExample?.path"
      router
      class="w-full min-h-screen"
    >
      <ElMenuItem
        v-for="example in examples"
        :key="example.name"
        :index="example.path"
      >
        <ElIcon>
          <component :is="example.icon" />
        </ElIcon>
        <span>{{ example.name }}</span>
      </ElMenuItem>
    </ElMenu>
  </ElScrollbar>
  <main
    class="flex-1 overflow-auto min-h-0 max-h-full"
  >
    <RouterView />
  </main>
</template>

<style module>
.aside {
  width: 220px;
}
</style>
