<script setup lang="ts">
import { GitBranch, Hash } from 'lucide-vue-next'
import { computed, markRaw } from 'vue'
import { useRoute } from 'vue-router'

const route = useRoute()

const examples = computed(() => {
  return [
    {
      name: '计算文件哈希值',
      path: '/file-hash',
      icon: markRaw(Hash),
      isActive: route.path.startsWith('/file-hash'),
    },
    {
      name: 'Git 仓库克隆',
      path: '/repo-clone',
      icon: markRaw(GitBranch),
      isActive: route.path.startsWith('/repo-clone'),
    },
  ]
})

const activeExample = computed(() => {
  return examples.value.find(example => example.isActive)
})
</script>

<template>
  <div class="flex h-screen min-h-0">
    <ElMenu
      :default-active="activeExample?.path"
      class="h-full"
      router
      :class="$style.aside"
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
    <main
      class="flex-1 overflow-auto min-h-0 max-h-full relative transition-colors duration-200"
    >
      <RouterView />
    </main>
  </div>
</template>

<style module>
.aside {
  width: 220px;
}
</style>
