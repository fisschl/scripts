<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import type { S3Object } from './components/s3-files'
import type { FileInfo } from '@/pages/file-copy/components/file-operations'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge } from 'lodash-es'
import { CloudUpload, Database, Folder } from 'lucide-vue-next'
import { object, string } from 'zod/mini'
import { listFilesRecursive } from '@/pages/file-copy/components/file-operations'
import { listRemoteFilesRecursive } from './components/s3-files'
import S3InstanceSelector from './components/S3InstanceSelector.vue'

/**
 * 保存的表单数据 Zod 模式定义
 */
const FormDataZod = object({
  /** 存储桶名称 */
  bucket: string(),
  /** S3 服务的终端节点 URL */
  endpoint_url: string(),
  /** 本地目录路径 */
  local_dir: string(),
  /** 远程目录路径 */
  remote_dir: string(),
})

/** 表单数据类型 */
interface FormData extends Infer<typeof FormDataZod> {}

// 同步操作类型
interface SyncOperation {
  type: 'upload' | 'delete'
  localPath?: string
  s3Key: string
}

// 表单数据
const form = reactive<FormData>({
  bucket: '',
  endpoint_url: '',
  local_dir: '',
  remote_dir: '',
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  endpoint_url: [
    { required: true, message: '请选择 S3 实例' },
  ],
  bucket: [
    { required: true, message: '请输入存储桶名称' },
  ],
  local_dir: [
    { required: true, message: '请选择本地目录' },
  ],
  remote_dir: [
    { required: true, message: '请输入远程目录路径' },
  ],
})

const loading = ref(false)
const drawerVisible = ref(false)
const currentStep = ref(0)
const uploadProgress = ref(0)

// 步骤定义
const steps = [
  { title: '准备上传', description: '验证配置和扫描文件' },
  { title: '扫描文件', description: '分析本地和远程文件' },
  { title: '上传文件', description: '同步文件到S3存储桶' },
  { title: '完成', description: '上传操作完成' },
]

const FORM_STORAGE_KEY = 's3-upload-form'
const store = Store.load('form-data.json')

/**
 * 初始化时加载保存的表单数据
 */
store.then(async (store) => {
  const data = await store.get(FORM_STORAGE_KEY)
  const result = FormDataZod.safeParse(data)
  if (!result.success) {
    return
  }
  merge(form, result.data)
  // 数据回显后移除表单校验状态
  await nextTick()
  formRef.value?.clearValidate()
})

/**
 * 获取本地文件列表
 *
 * 递归扫描指定目录下的所有文件，返回相对路径到文件信息的映射。
 * 自动处理路径分隔符转换，并过滤出文件类型（排除目录）。
 *
 * @param dir - 要扫描的本地目录路径
 * @returns Promise<Map<string, FileInfo>> 返回相对路径到文件信息的映射
 *
 * @throws {Error} 当目录扫描失败时抛出错误
 */
async function getLocalFiles(dir: string): Promise<Map<string, FileInfo>> {
  const localFiles = new Map<string, FileInfo>()

  try {
    // 获取所有文件的完整信息
    const allFiles = await listFilesRecursive(dir)

    for (const fileInfo of allFiles) {
      // 移除开头的路径分隔符并统一为正斜杠
      const relativePath = fileInfo.path
        .replace(dir, '')
        .replace(/^[\\/]+/, '')
        .replace(/[\\/]+/g, '/')

      localFiles.set(relativePath, fileInfo)
    }

    return localFiles
  }
  catch (error) {
    throw new Error(`扫描本地目录失败: ${error}`)
  }
}

/**
 * 生成同步操作队列
 *
 * 比较本地文件和远程文件，生成需要执行的同步操作。
 * 采用覆盖式同步策略：本地存在的文件总是上传，远程存在但本地不存在的文件将被删除。
 *
 * @param localFiles - 本地文件映射（相对路径 -> 文件信息）
 * @param remoteFiles - 远程文件映射（相对路径 -> S3对象信息）
 * @param prefix - S3 对象键前缀
 * @returns SyncOperation[] 返回同步操作队列
 */
function generateSyncOperations(
  localFiles: Map<string, FileInfo>,
  remoteFiles: Map<string, S3Object>,
  prefix: string,
): SyncOperation[] {
  const operations: SyncOperation[] = []

  // 1. 检查需要上传的文件（本地存在，远程不存在或需要覆盖）
  for (const [relativePath, localFile] of localFiles) {
    const remoteFile = remoteFiles.get(relativePath)
    const s3Key = prefix + relativePath

    if (!remoteFile) {
      // 远程不存在，需要上传
      operations.push({
        type: 'upload',
        localPath: localFile.path,
        s3Key,
      })
    }
    else {
      // 远程存在，可以选择比较文件大小或修改时间来决定是否覆盖
      // 这里简单实现为总是覆盖（覆盖式同步）
      operations.push({
        type: 'upload',
        localPath: localFile.path,
        s3Key,
      })
    }
  }

  // 2. 检查需要删除的远程文件（远程存在，本地不存在）
  for (const [relativePath] of remoteFiles) {
    if (!localFiles.has(relativePath)) {
      operations.push({
        type: 'delete',
        s3Key: prefix + relativePath,
      })
    }
  }

  return operations
}

/**
 * 执行同步操作
 *
 * 按顺序执行同步操作队列中的所有操作，支持上传和删除操作。
 * 实时更新进度信息，遇到错误时立即停止并抛出异常。
 *
 * @param operations - 同步操作队列
 * @param endpointUrl - S3 服务的终端节点 URL
 * @param bucket - 存储桶名称
 *
 * @throws {Error} 当同步操作失败时抛出错误
 */
async function executeSyncOperations(
  operations: SyncOperation[],
  endpointUrl: string,
  bucket: string,
) {
  currentStep.value = 2 // 切换到上传步骤

  for (let i = 0; i < operations.length; i++) {
    const operation = operations[i]
    if (!operation)
      continue

    try {
      if (operation.type === 'upload' && operation.localPath) {
        await invoke('upload_file', {
          endpoint_url: endpointUrl,
          bucket,
          localPath: operation.localPath,
          s3Key: operation.s3Key,
        })
      }
      else if (operation.type === 'delete') {
        await invoke('delete_object', {
          endpoint_url: endpointUrl,
          bucket,
          s3Key: operation.s3Key,
        })
      }

      // 更新进度
      uploadProgress.value = Math.round(((i + 1) / operations.length) * 100)
    }
    catch (error) {
      const errorMsg = `${operation.type === 'upload' ? '上传' : '删除'}失败: ${operation.s3Key} - ${error}`
      throw new Error(errorMsg)
    }
  }
}

/**
 * 选择本地目录
 *
 * 打开目录选择对话框，让用户选择要上传的本地目录。
 */
async function selectLocalDir() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.local_dir = selected
}

/**
 * 开始上传流程
 *
 * 执行完整的 S3 同步上传流程：
 * 1. 验证表单并保存配置
 * 2. 扫描本地文件
 * 3. 获取远程文件列表
 * 4. 生成同步操作队列
 * 5. 执行同步操作
 *
 * @throws {Error} 当上传过程中出现错误时抛出异常
 */
async function startUpload() {
  await formRef.value?.validate()

  // 保存表单数据（包含所有信息）
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, form)
    await store.save()
  })

  loading.value = true
  drawerVisible.value = true
  currentStep.value = 0
  uploadProgress.value = 0

  try {
    const remotePrefix = (`${form.remote_dir}/`).replace(/^[\\/]+/, '').replace(/[\\/]+/g, '/')

    currentStep.value = 1 // 切换到扫描文件步骤

    // 1. 获取本地文件列表
    const localFiles = await getLocalFiles(form.local_dir)

    // 2. 获取远程文件列表
    const remoteObjects = await listRemoteFilesRecursive(
      form.endpoint_url,
      form.bucket,
      remotePrefix,
    )

    // 3. 转换为相对路径映射
    const remoteFiles = new Map<string, S3Object>()
    for (const obj of remoteObjects) {
      const relativeKey = obj.key.replace(remotePrefix, '')
      if (relativeKey) {
        remoteFiles.set(relativeKey, obj)
      }
    }

    // 4. 生成同步操作队列
    const operations = generateSyncOperations(localFiles, remoteFiles, remotePrefix)

    if (operations.length === 0) {
      currentStep.value = 3 // 直接跳到完成步骤
      ElMessage.success('本地和远程文件完全一致，无需同步')
      return
    }

    // 5. 执行同步操作
    await executeSyncOperations(operations, form.endpoint_url, form.bucket)

    currentStep.value = 3 // 完成步骤
    ElMessage.success('S3 上传完成')
  }
  catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    ElMessage.error(`S3 上传失败: ${errorMsg}`)
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
      @submit.prevent="startUpload"
    >
      <ElFormItem label="S3 实例" prop="endpoint_url">
        <S3InstanceSelector
          v-model="form.endpoint_url"
          :class="$style.input"
          :disabled="loading"
        />
      </ElFormItem>

      <ElFormItem label="存储桶名称 (Bucket)" prop="bucket">
        <ElInput
          v-model.trim="form.bucket"
          placeholder="请输入 S3 存储桶名称"
          :class="$style.bucketInput"
          :disabled="loading"
        >
          <template #prefix>
            <Database :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElDivider />

      <ElFormItem label="本地目录" prop="local_dir">
        <ElInput
          v-model="form.local_dir"
          placeholder="点击选择要上传的本地目录..."
          :class="$style.input"
          :disabled="loading"
          @click="selectLocalDir"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="远程目录路径" prop="remote_dir">
        <ElInput
          v-model.trim="form.remote_dir"
          placeholder="例如: website/ 或 backup/2024/ (建议以斜杠结尾)"
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <div v-if="!loading" class="mt-4">
        <ElButton type="primary" native-type="submit">
          <CloudUpload :size="18" class="mr-2" />
          开始上传
        </ElButton>
      </div>
    </ElForm>

    <!-- 抽屉组件 -->
    <ElDrawer
      v-model="drawerVisible"
      title="S3 上传进度"
      direction="rtl"
      size="400px"
      :close-on-click-modal="false"
      :close-on-press-escape="false"
    >
      <div class="p-4">
        <!-- 步骤条 -->
        <ElSteps :active="currentStep" direction="vertical" finish-status="success">
          <ElStep
            v-for="(step, index) in steps"
            :key="index"
            :title="step.title"
            :description="step.description"
          />
        </ElSteps>

        <!-- 上传进度条 -->
        <div v-if="currentStep === 2" class="mt-6">
          <div class="mb-2 text-sm text-gray-600">
            上传进度
          </div>
          <ElProgress :percentage="uploadProgress" :stroke-width="8" />
        </div>

        <!-- 完成状态 -->
        <div v-if="currentStep === 3" class="mt-6 text-center">
          <ElResult
            icon="success"
            title="上传完成"
            sub-title="所有文件已成功上传到 S3 存储桶"
          />
          <ElButton type="primary" class="mt-4" @click="drawerVisible = false">
            关闭
          </ElButton>
        </div>
      </div>
    </ElDrawer>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}

.bucketInput {
  max-width: 20rem;
}

.regionInput {
  max-width: 20rem;
}
</style>
