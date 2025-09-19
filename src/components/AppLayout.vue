<script setup lang="ts">
import { Hash } from 'lucide-vue-next'
import { markRaw } from 'vue'

const examples = [
  {
    name: '计算文件哈希值',
    path: '/file-hash',
    icon: markRaw(Hash),
  },
]
</script>

<template>
  <div class="flex h-screen bg-neutral-100 dark:bg-neutral-900">
    <aside class="w-72 h-screen bg-neutral-800 dark:bg-neutral-950 text-gray-800 dark:text-white flex flex-col border-r border-neutral-200 dark:border-neutral-800">
      <nav class="flex-1 overflow-y-auto">
        <RouterLink
          v-for="example in examples"
          :key="example.name"
          :to="example.path"
          class="block p-3 px-5 cursor-pointer transition-all duration-200 ease-in-out border-l-3 border-transparent no-underline text-inherit hover:bg-neutral-200 dark:hover:bg-neutral-800 hover:border-l-blue-500"
          active-class="bg-neutral-200 dark:bg-neutral-800 border-l-blue-500"
        >
          <component :is="example.icon" class="w-4 h-4 mr-2 flex-shrink-0" />
          <span class="text-sm font-medium">
            {{ example.name }}
          </span>
        </RouterLink>
      </nav>
    </aside>
    <main class="flex-1 overflow-hidden relative bg-neutral-100 dark:bg-neutral-900 transition-colors duration-200">
      <RouterView />
    </main>
  </div>
</template>
