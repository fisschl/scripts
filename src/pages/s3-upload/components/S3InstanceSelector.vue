<script setup lang="ts">
import type { S3Instance } from './instances'
import { HardDrive } from 'lucide-vue-next'
import { loadS3Instances } from './instances'

interface Props {
  placeholder?: string
  disabled?: boolean
  class?: string
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: '请选择已配置的 S3 实例',
  disabled: false,
})

const modelValue = defineModel<string>({ default: '' })

// S3 实例列表
const s3Instances = ref<S3Instance[]>([])

// 加载 S3 实例列表
async function loadInstances() {
  s3Instances.value = await loadS3Instances()
}

// 组件挂载时加载 S3 实例
onMounted(() => {
  loadInstances()
})
</script>

<template>
  <ElSelect
    v-model="modelValue"
    :placeholder="placeholder"
    :class="props.class"
    :disabled="disabled"
    filterable
  >
    <template #prefix>
      <HardDrive :size="16" />
    </template>
    <ElOption
      v-for="instance in s3Instances"
      :key="instance.endpoint_url"
      :label="instance.endpoint_url"
      :value="instance.endpoint_url"
    />
  </ElSelect>
</template>
