<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { File, Key } from 'lucide-vue-next'

const filePath = ref('')
const hashResult = ref('')

async function selectFile() {
  const selected = await open({
    multiple: false,
    directory: false,
  })
  if (!selected)
    return
  filePath.value = selected
  if (!filePath.value)
    return
  const result = await invoke('calculate_file_hash', { filePath: filePath.value })
  if (typeof result !== 'string')
    return
  hashResult.value = result
}
</script>

<template>
  <div class="p-4">
    <ElForm label-position="top" label-suffix="：">
      <ElFormItem label="选择文件">
        <ElInput
          v-model.trim="filePath"
          placeholder="点击选择文件..."
          :class="$style.input"
          @click="selectFile"
        >
          <template #prefix>
            <File :size="18" />
          </template>
        </ElInput>
      </ElFormItem>
      <ElFormItem v-if="hashResult" label="哈希结果">
        <ElInput
          :value="hashResult"
          :class="$style.input"
        >
          <template #prefix>
            <Key :size="18" />
          </template>
        </ElInput>
      </ElFormItem>
    </ElForm>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
