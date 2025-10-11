<script setup lang="ts">
import type { FileItem } from './types'
import { Loader2 } from 'lucide-vue-next'

/**
 * 文件列表项组件属性
 */
interface Props {
  /** 文件项信息 */
  file: FileItem
}

const props = defineProps<Props>()

/**
 * 从文件路径中提取文件名
 */
const fileName = computed(() => {
  return props.file.path.split(/[/\\]/).pop() || '未知文件'
})
</script>

<template>
  <li class="pb-2 flex items-center gap-2">
    <Loader2
      v-if="file.status === 'processing'"
      :size="16"
      class="animate-spin text-blue-500"
    />
    <span class="truncate">
      {{ fileName }}
    </span>
  </li>
</template>
