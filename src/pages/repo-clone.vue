<script setup lang="ts">
import type { FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ElMessage } from 'element-plus'
import { GitBranch } from 'lucide-vue-next'
import { onBeforeUnmount, onMounted, reactive, ref, useTemplateRef } from 'vue'

// 表单数据
const form = reactive({
  sourceUrl: '',
  targetUrl: '',
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  sourceUrl: [
    { required: true, message: '请输入源仓库URL', trigger: 'blur' },
    { type: 'url', message: '请输入有效的URL地址', trigger: 'blur' },
  ],
  targetUrl: [
    { required: true, message: '请输入目标仓库URL', trigger: 'blur' },
    { type: 'url', message: '请输入有效的URL地址', trigger: 'blur' },
  ],
})

const loading = ref(false)
const progressMessages = ref<string[]>([])

async function startClone() {
  await formRef.value?.validate()

  loading.value = true
  progressMessages.value = ['开始仓库克隆操作...']

  try {
    await invoke('repo_mirror', {
      from: form.sourceUrl,
      to: form.targetUrl,
    })
  }
  catch (error: unknown) {
    loading.value = false
    progressMessages.value.push(`❌ 错误: ${error}`)
    ElMessage.error(`克隆失败: ${error}`)
  }
  finally {
    loading.value = false
  }

  ElMessage.success('仓库克隆操作完成')
}

const effects: (() => unknown)[] = []

onMounted(async () => {
  effects.push(
    await listen('repo-mirror-info', (event) => {
      const { payload } = event
      if (typeof payload !== 'string')
        return
      progressMessages.value.push(payload)
    }),
  )
})

onBeforeUnmount(() => {
  effects.forEach(effect => effect())
  effects.length = 0
})
</script>

<template>
  <div class="p-6">
    <ElForm
      ref="form-ref"
      :model="form"
      :rules="rules"
      label-position="top"
      label-suffix="："
      @submit.prevent="startClone"
    >
      <ElFormItem label="源仓库 URL" prop="sourceUrl">
        <ElInput
          v-model="form.sourceUrl"
          placeholder="https://github.com/username/repo.git"
          :class="$style.input"
          :disabled="loading"
        />
      </ElFormItem>

      <ElFormItem label="目标仓库 URL" prop="targetUrl">
        <ElInput
          v-model="form.targetUrl"
          placeholder="https://gitlab.com/username/repo.git"
          :class="$style.input"
          :disabled="loading"
        />
      </ElFormItem>

      <div class="mt-4">
        <ElButton type="primary" :loading="loading" native-type="submit">
          <template #icon>
            <GitBranch />
          </template>
          开始克隆
        </ElButton>
      </div>
    </ElForm>
    <ol class="py-6 space-y-1 text-gray-500 dark:text-gray-400">
      <li v-for="(message, index) in progressMessages" :key="index">
        {{ message }}
      </li>
    </ol>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
