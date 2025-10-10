<script setup lang="ts">
import type { FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { ExternalLink, GitBranch, Link } from 'lucide-vue-next'
import LogViewer from '@/components/LogViewer.vue'

// 类型定义
interface CommandResult {
  exit_code: number | null
  stdout: string
  stderr: string
}

interface ProgressInfo {
  message: string
  progress: number // 0.0 - 1.0
}

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
    { required: true, message: '请输入源仓库URL' },
    { type: 'url', message: '请输入有效的URL地址' },
  ],
  targetUrl: [
    { required: true, message: '请输入目标仓库URL' },
    { type: 'url', message: '请输入有效的URL地址' },
  ],
})

const loading = ref(false)
const progressMessages = ref<string[]>([])

/**
 * 验证源仓库是否可访问
 */
async function validateRepository(source: string): Promise<void> {
  const result = await invoke<CommandResult>('execute_command_sync', {
    command: 'git',
    args: ['ls-remote', source],
  })

  if (result.exit_code !== 0) {
    throw new Error(result.stderr || '源仓库不可访问')
  }
}

/**
 * 解析 Git 输出中的进度信息
 */
function parseGitProgress(output: string): ProgressInfo | null {
  const patterns = [
    {
      pattern: /Counting objects:\s+(\d+)%/,
      phase: '计数对象',
      baseProgress: 0.1,
    },
    {
      pattern: /Compressing objects:\s+(\d+)%/,
      phase: '压缩对象',
      baseProgress: 0.3,
    },
    {
      pattern: /Receiving objects:\s+(\d+)%/,
      phase: '接收对象',
      baseProgress: 0.5,
    },
    {
      pattern: /Resolving deltas:\s+(\d+)%/,
      phase: '解析差异',
      baseProgress: 0.8,
    },
    {
      pattern: /Enumerating objects:\s+(\d+)%/,
      phase: '枚举对象',
      baseProgress: 0.1,
    },
  ]

  for (const { pattern, phase, baseProgress } of patterns) {
    const match = output.match(pattern)
    if (match) {
      const percentage = Number.parseInt(match[1]!) / 100
      return {
        message: `${phase}: ${match[1]}%`,
        progress: baseProgress + (percentage * 0.2), // 每个阶段占 20% 进度
      }
    }
  }

  return null
}

/**
 * 执行 Git 克隆命令
 */
async function executeGitClone(
  source: string,
  target: string,
  onProgress?: (info: ProgressInfo) => void,
): Promise<void> {
  // 使用 --mirror 参数进行镜像克隆
  const result = await invoke<CommandResult>('execute_command_sync', {
    command: 'git',
    args: ['clone', '--mirror', source, target],
  })

  if (result.exit_code !== 0) {
    throw new Error(result.stderr || 'Git 克隆失败')
  }

  // 如果有输出，尝试解析进度信息
  if (result.stdout && onProgress) {
    const lines = result.stdout.split('\n')
    for (const line of lines) {
      const progress = parseGitProgress(line)
      if (progress) {
        onProgress(progress)
      }
    }
  }
}

/**
 * 主要的仓库镜像函数
 */
async function mirrorRepository(
  source: string,
  target: string,
  onProgress?: (info: ProgressInfo) => void,
): Promise<void> {
  try {
    // 1. 验证源仓库
    onProgress?.({ message: '验证源仓库...', progress: 0.1 })
    await validateRepository(source)

    // 2. 执行克隆
    onProgress?.({ message: '开始克隆仓库...', progress: 0.3 })
    await executeGitClone(source, target, onProgress)

    // 3. 完成
    onProgress?.({ message: '仓库镜像完成', progress: 1.0 })
  }
  catch (error) {
    onProgress?.({
      message: `镜像失败: ${error instanceof Error ? error.message : String(error)}`,
      progress: -1,
    })
    throw error
  }
}

/**
 * 简化版本的仓库镜像函数（仅克隆）
 */
async function simpleMirrorRepository(
  source: string,
  target: string,
  onProgress?: (message: string) => void,
): Promise<void> {
  const progressCallback = onProgress
    ? (info: ProgressInfo) => onProgress(info.message)
    : undefined

  await mirrorRepository(source, target, progressCallback)
}

async function startClone() {
  await formRef.value?.validate()

  loading.value = true
  progressMessages.value = []

  try {
    await simpleMirrorRepository(
      form.sourceUrl,
      form.targetUrl,
      (message: string) => {
        progressMessages.value.push(message)
      },
    )

    ElMessage.success('仓库克隆操作完成')
  }
  catch (error: unknown) {
    progressMessages.value.push(`错误: ${error}`)
    ElMessage.error(`克隆失败: ${error}`)
  }
  finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="p-4">
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
          v-model.trim="form.sourceUrl"
          placeholder="https://github.com/username/repo.git"
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <Link :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="目标仓库 URL" prop="targetUrl">
        <ElInput
          v-model.trim="form.targetUrl"
          placeholder="https://gitlab.com/username/repo.git"
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <ExternalLink :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <div class="mt-4">
        <ElButton type="primary" :loading="loading" native-type="submit">
          <GitBranch :size="18" class="mr-2" />
          开始克隆
        </ElButton>
      </div>
    </ElForm>
    <template v-if="progressMessages.length > 0">
      <ElDivider />
      <LogViewer :logs="progressMessages" class="mb-4" />
    </template>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
