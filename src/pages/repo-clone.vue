<script setup lang="ts">
import type { FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { join, tempDir } from '@tauri-apps/api/path'
import { ExternalLink, GitBranch, Link } from 'lucide-vue-next'

/**
 * 命令执行结果类型
 */
interface CommandResult {
  /** 退出代码 */
  exit_code: number | null
  /** 标准输出 */
  stdout: string
  /** 标准错误输出 */
  stderr: string
}

/**
 * 表单数据类型
 */
interface RepoCloneForm {
  /** 源仓库 URL */
  sourceUrl: string
  /** 目标仓库 URL */
  targetUrl: string
}

// 表单数据
const form = reactive<RepoCloneForm>({
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

/** 加载状态 */
const loading = ref(false)
/** 进度抽屉显示状态 */
const drawerVisible = ref(false)
/** 当前执行步骤 */
const currentStep = ref(0)

/** 克隆步骤定义 */
const steps = [
  '验证源仓库',
  '克隆仓库',
  '配置远程',
  '推送到新远程',
  '清理临时目录',
  '操作完成',
]

/**
 * 开始仓库克隆流程
 *
 * 执行完整的仓库镜像克隆和推送流程：
 * 1. 验证源仓库可访问性
 * 2. 镜像克隆到临时目录
 * 3. 配置目标远程地址
 * 4. 推送到目标仓库
 * 5. 清理临时文件
 */
async function startClone() {
  await formRef.value?.validate()

  loading.value = true
  drawerVisible.value = true
  currentStep.value = 0

  try {
    // 步骤1：验证源仓库
    currentStep.value = 1
    await new Promise(resolve => setTimeout(resolve, 500))
    await invoke<CommandResult>('execute_command_sync', {
      command: 'git',
      args: ['ls-remote', form.sourceUrl],
      workingDir: await tempDir(),
    })

    // 步骤2：克隆仓库到临时目录
    currentStep.value = 2
    await new Promise(resolve => setTimeout(resolve, 500))

    // 从目标URL提取仓库名作为临时目录名
    const repoName = form.targetUrl.split('/').pop()?.replace('.git', '') || 'temp-repo'
    const systemTempDir = await tempDir()
    const tempPath = await join(systemTempDir, repoName)

    await invoke<CommandResult>('execute_command_sync', {
      command: 'git',
      args: ['clone', '--mirror', form.sourceUrl, tempPath],
      workingDir: systemTempDir,
    })

    // 步骤3：配置新的远程推送地址
    currentStep.value = 3
    await new Promise(resolve => setTimeout(resolve, 500))

    await invoke<CommandResult>('execute_command_sync', {
      command: 'git',
      args: ['remote', 'add', 'target', form.targetUrl],
      workingDir: tempPath,
    })

    // 步骤4：推送到新远程
    currentStep.value = 4
    await new Promise(resolve => setTimeout(resolve, 500))

    await invoke<CommandResult>('execute_command_sync', {
      command: 'git',
      args: ['push', '--mirror', 'target'],
      workingDir: tempPath,
    })

    // 步骤5：清理临时目录
    currentStep.value = 5
    await new Promise(resolve => setTimeout(resolve, 500))

    // 使用 Rust 后端递归删除临时目录
    await invoke('remove_path', {
      path: tempPath,
    })

    // 步骤6：操作完成
    currentStep.value = 6

    // 等待1秒后关闭抽屉
    await new Promise(resolve => setTimeout(resolve, 1000))
    drawerVisible.value = false

    ElMessage.success('仓库镜像和推送完成')
  }
  catch (error) {
    drawerVisible.value = false
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

    <!-- 进度抽屉 -->
    <ElDrawer
      v-model="drawerVisible"
      title="仓库克隆进度"
      direction="rtl"
      size="400px"
      :with-header="true"
      :show-close="false"
      :modal="true"
      :close-on-click-modal="false"
    >
      <ElSteps
        :active="currentStep"
        direction="vertical"
        finish-status="success"
        align-center
        class="px-6"
      >
        <ElStep
          v-for="(step, index) in steps"
          :key="index"
          :title="step"
        />
      </ElSteps>
    </ElDrawer>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
