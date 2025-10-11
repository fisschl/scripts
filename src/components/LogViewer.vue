<script setup lang="ts">
/**
 * 日志查看器组件属性
 */
interface Props {
  /** 日志消息数组 */
  logs: string[]
}

const props = defineProps<Props>()

/**
 * 获取日志容器引用
 */
const logContainer = useTemplateRef('log-container')

/**
 * 滚动到日志容器末尾
 */
async function scrollToBottom() {
  await nextTick()
  const element = logContainer.value
  if (!element)
    return
  element.scrollTop = element.scrollHeight
}

/**
 * 获取最后一条日志
 */
const lastLog = computed(() => props.logs.at(-1))

/**
 * 监听日志变化，当最后一个字符串变化时自动滚动到底部
 */
watch(lastLog, (value, oldValue) => {
  if (value === oldValue)
    return
  scrollToBottom()
})

/**
 * 组件挂载后如果有初始日志也滚动到末尾
 */
onMounted(() => {
  if (lastLog.value)
    scrollToBottom()
})
</script>

<template>
  <ol
    ref="log-container"
    class="py-1 logs-container overflow-y-auto text-sm font-mono text-gray-700 dark:text-gray-300 space-y-1"
  >
    <li v-for="(log, index) in props.logs" :key="index" class="break-words">
      {{ log }}
    </li>
  </ol>
</template>

<style scoped>
/* 日志容器高度 */
.logs-container {
  max-height: 80vh;
  /* 隐藏滚动条 */
  scrollbar-width: none; /* Firefox */
}

.logs-container::-webkit-scrollbar {
  display: none; /* Chrome, Safari and Opera */
}
</style>
