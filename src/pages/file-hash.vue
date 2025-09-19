<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { File, Key } from 'lucide-vue-next'
import { ref } from 'vue'

const filePath = ref('')
const hashResult = ref('')
const isLoading = ref(false)
const errorMessage = ref('')

async function selectFile() {
  try {
    const selected = await open({
      multiple: false,
      filters: [{
        name: 'All Files',
        extensions: ['*'],
      }],
    })

    if (selected) {
      filePath.value = selected
      errorMessage.value = ''
    }
  }
  catch (error) {
    errorMessage.value = `选择文件失败: ${error}`
  }
}

async function calculateHash() {
  if (!filePath.value) {
    errorMessage.value = '请先选择文件'
    return
  }

  isLoading.value = true
  hashResult.value = ''
  errorMessage.value = ''

  try {
    const result = await invoke('calculate_file_hash', { filePath: filePath.value })
    hashResult.value = result as string
  }
  catch (error) {
    errorMessage.value = `计算哈希值失败: ${error}`
  }
  finally {
    isLoading.value = false
  }
}

async function copyToClipboard() {
  if (!hashResult.value)
    return

  try {
    await navigator.clipboard.writeText(hashResult.value)
    // 哈希值已成功复制到剪贴板
  }
  catch (error) {
    errorMessage.value = `复制失败: ${error}`
  }
}

function clearAll() {
  filePath.value = ''
  hashResult.value = ''
  errorMessage.value = ''
}
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto space-y-6">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white">
      文件哈希值计算
    </h1>

    <ElCard class="border border-gray-200 dark:border-gray-600">
      <div class="space-y-4">
        <div>
          <ElFormItem label="选择文件">
            <div class="flex gap-2">
              <ElInput
                v-model="filePath"
                readonly
                placeholder="点击选择文件..."
                class="flex-1"
              >
                <template #prefix>
                  <ElIcon>
                    <File />
                  </ElIcon>
                </template>
              </ElInput>
              <ElButton type="primary" @click="selectFile">
                选择文件
              </ElButton>
            </div>
          </ElFormItem>
        </div>

        <div class="flex gap-2">
          <ElButton
            type="success"
            :loading="isLoading"
            :disabled="!filePath"
            @click="calculateHash"
          >
            {{ isLoading ? '计算中...' : '计算哈希值' }}
          </ElButton>

          <ElButton @click="clearAll">
            清空
          </ElButton>
        </div>

        <div v-if="hashResult">
          <ElFormItem label="哈希结果 (Blake3 + Base32)">
            <div class="flex items-center gap-2">
              <ElInput
                :value="hashResult"
                readonly
                class="flex-1 font-mono text-sm"
              >
                <template #prefix>
                  <ElIcon>
                    <Key />
                  </ElIcon>
                </template>
              </ElInput>
              <ElButton type="primary" size="small" @click="copyToClipboard">
                复制
              </ElButton>
            </div>
          </ElFormItem>
        </div>

        <ElAlert
          v-if="errorMessage"
          :title="errorMessage"
          type="error"
          show-icon
          :closable="false"
        />
      </div>
    </ElCard>

    <ElCard class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800">
      <template #header>
        <div class="text-lg font-semibold text-blue-900 dark:text-blue-100">
          功能说明
        </div>
      </template>
      <ul class="text-blue-800 dark:text-blue-200 text-sm space-y-1">
        <li>• 使用高性能的 Blake3 哈希算法</li>
        <li>• 结果使用 Base32-Crockford 编码，便于阅读和分享</li>
        <li>• 支持大文件处理，内存占用低</li>
        <li>• 可用于文件完整性验证和唯一标识</li>
      </ul>
    </ElCard>
  </div>
</template>
