<script setup lang="ts">
import type { S3Instance } from './instances'
import { HardDrive, Settings } from 'lucide-vue-next'
import { loadS3Instances } from './instances'
import S3ConfigDrawer from './S3ConfigDrawer.vue'

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

// 控制抽屉显示
const showConfigDrawer = ref(false)

// 加载 S3 实例列表
async function loadInstances() {
  s3Instances.value = await loadS3Instances()
}

loadInstances()

// 打开配置抽屉
function openConfigDrawer() {
  showConfigDrawer.value = true
}

// 配置关闭后刷新实例列表
function onConfigDrawerClose() {
  loadInstances()
}
</script>

<template>
  <div class="flex items-center gap-2">
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
        :key="instance.s3_instance_id"
        :label="instance.endpoint_url"
        :value="instance.s3_instance_id"
      />
    </ElSelect>

    <ElButton
      :disabled="disabled"
      circle
      size="default"
      title="S3 实例配置"
      @click="openConfigDrawer"
    >
      <Settings :size="16" />
    </ElButton>

    <!-- S3 配置抽屉 -->
    <S3ConfigDrawer
      v-model="showConfigDrawer"
      @close="onConfigDrawerClose"
    />
  </div>
</template>
