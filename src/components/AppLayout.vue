<script setup lang="ts">
import { ElScrollbar } from 'element-plus'
import { CloudUpload, Copy, GitBranch, Hash } from 'lucide-vue-next'
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
      name: 'S3 上传',
      path: '/s3-upload',
      icon: markRaw(CloudUpload),
      isActive: route.path.startsWith('/s3-upload'),
    },
  ]
})

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
