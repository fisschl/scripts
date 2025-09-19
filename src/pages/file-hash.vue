<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
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
      filePath.value = selected as string
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
    alert('哈希值已复制到剪贴板')
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
  <div class="p-6 max-w-4xl mx-auto">
    <h1 class="text-2xl font-bold text-gray-900 dark:text-white mb-6">
      文件哈希值计算
    </h1>

    <div class="bg-white dark:bg-neutral-800 rounded-lg shadow-md p-6 mb-6">
      <div class="mb-4">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          选择文件
        </label>
        <div class="flex gap-2">
          <input
            v-model="filePath"
            type="text"
            readonly
            placeholder="点击选择文件..."
            class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-gray-50 dark:bg-neutral-700 text-gray-900 dark:text-white"
          >
          <button
            class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
            @click="selectFile"
          >
            选择文件
          </button>
        </div>
      </div>

      <div class="flex gap-2 mb-4">
        <button
          :disabled="isLoading || !filePath"
          class="px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors"
          @click="calculateHash"
        >
          {{ isLoading ? '计算中...' : '计算哈希值' }}
        </button>

        <button
          class="px-4 py-2 bg-gray-500 text-white rounded-md hover:bg-gray-600 transition-colors"
          @click="clearAll"
        >
          清空
        </button>
      </div>

      <div v-if="hashResult" class="mt-6">
        <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          哈希结果 (Blake3 + Base32)
        </label>
        <div class="flex items-center gap-2">
          <input
            :value="hashResult"
            type="text"
            readonly
            class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md bg-gray-50 dark:bg-neutral-700 text-gray-900 dark:text-white font-mono text-sm"
          >
          <button
            class="px-3 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors text-sm"
            @click="copyToClipboard"
          >
            复制
          </button>
        </div>
      </div>

      <div v-if="errorMessage" class="mt-4 p-3 bg-red-100 dark:bg-red-900 border border-red-300 dark:border-red-700 rounded-md">
        <p class="text-red-700 dark:text-red-300 text-sm">
          {{ errorMessage }}
        </p>
      </div>
    </div>

    <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
      <h3 class="text-lg font-semibold text-blue-900 dark:text-blue-100 mb-2">
        功能说明
      </h3>
      <ul class="text-blue-800 dark:text-blue-200 text-sm space-y-1">
        <li>• 使用高性能的 Blake3 哈希算法</li>
        <li>• 结果使用 Base32-Crockford 编码，便于阅读和分享</li>
        <li>• 支持大文件处理，内存占用低</li>
        <li>• 可用于文件完整性验证和唯一标识</li>
      </ul>
    </div>
  </div>
</template>
