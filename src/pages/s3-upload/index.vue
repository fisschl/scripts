<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import type { S3Object } from './components/s3-files'
import type { FileInfo } from '@/pages/file-copy/components/file-operations'
import { invoke } from '@tauri-apps/api/core'
import { join } from '@tauri-apps/api/path'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge } from 'lodash-es'
import { CloudUpload, Database, Folder } from 'lucide-vue-next'
import { boolean, object, string } from 'zod/mini'
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
  /** 是否删除远程多余文件 */
  delete_remote_extras: boolean(),
})

/** 表单数据类型 */
interface FormData extends Infer<typeof FormDataZod> {}

// 表单数据
const form = reactive<FormData>({
  bucket: '',
  endpoint_url: '',
  local_dir: '',
  remote_dir: '',
  delete_remote_extras: true, // 默认勾选删除远程多余文件
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

// 响应式路径集合
const localPaths = reactive<Set<string>>(new Set())
const remotePaths = reactive<Set<string>>(new Set())

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
 * 获取本地文件路径列表
 *
 * 递归扫描指定目录下的所有文件，追加到响应式集合。
 * 自动处理路径分隔符转换，并过滤出文件类型（排除目录）。
 *
 * @param dir - 要扫描的本地目录路径
 */
async function updateLocalFilePaths(dir: string): Promise<void> {
  // 递归扫描目录的内部函数
  async function scanDirectory(dirPath: string): Promise<void> {
    const files = await invoke<FileInfo[]>('list_directory', { path: dirPath })

    for (const file of files) {
      if (file.is_dir) {
        // 递归处理子目录
        await scanDirectory(file.path)
      }
      else {
        // 移除开头的路径分隔符并统一为正斜杠
        const relativePath = file.path
          .replace(dir, '')
          .replace(/^[\\/]+/, '')
          .replace(/[\\/]+/g, '/')

        localPaths.add(relativePath)
      }
    }
  }

  await scanDirectory(dir)
}

/**
 * 获取远程文件路径列表
 *
 * 递归获取远程S3存储桶中的所有文件，追加到响应式集合。
 *
 * @param endpointUrl - S3 服务的终端节点 URL
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 */
async function updateRemoteFilePaths(
  endpointUrl: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  let continuationToken: string | undefined

  do {
    // 调用后端API获取一页对象
    const response = await invoke<{
      objects: S3Object[]
      is_truncated: boolean
      next_continuation_token?: string
    }>('list_objects', {
      endpoint_url: endpointUrl,
      bucket,
      prefix,
      continuation_token: continuationToken,
    })

    // 转换为相对路径并添加到响应式集合
    for (const obj of response.objects) {
      const relativePath = obj.key
        .replace(prefix, '')
        .replace(/^\/+/, '') // 移除开头的斜杠（S3路径只使用正斜杠）

      if (relativePath) {
        remotePaths.add(relativePath)
      }
    }

    // 检查是否还有更多数据
    continuationToken = response.next_continuation_token

    // 如果还有更多数据，添加短暂延迟
    if (continuationToken) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }
  } while (continuationToken)
}

/**
 * 上传本地文件到S3
 *
 * 遍历本地文件路径列表，上传文件到S3存储桶
 *
 * @param localDir - 本地目录路径
 * @param endpointUrl - S3 服务的终端节点 URL
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 * @throws {Error} 当上传失败时抛出错误
 */
async function uploadLocalFiles(
  localDir: string,
  endpointUrl: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  // 使用数组来遍历，避免在迭代过程中修改集合
  const pathsToUpload = Array.from(localPaths)

  for (const relativePath of pathsToUpload) {
    // 使用Tauri的join函数进行跨平台路径规范化拼接
    const localPath = await join(localDir, relativePath)
    const s3Key = prefix + relativePath

    await invoke('upload_file', {
      endpoint_url: endpointUrl,
      bucket,
      localPath,
      s3Key,
    })

    // 上传成功后，从本地集合中删除
    localPaths.delete(relativePath)
    // 同时从远程集合中删除（如果存在的话）
    remotePaths.delete(relativePath)
  }
}

/**
 * 删除远程多余文件
 *
 * 删除远程集合中剩余的所有文件（这些文件在本地不存在）
 *
 * @param endpointUrl - S3 服务的终端节点 URL
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 * @throws {Error} 当删除失败时抛出错误
 */
async function deleteRemoteExtraFiles(
  endpointUrl: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  // 上传完成后，remotePaths中剩下的就是需要删除的多余文件
  // 直接遍历并删除所有剩余文件
  const filesToDelete = Array.from(remotePaths)

  for (const relativePath of filesToDelete) {
    const s3Key = prefix + relativePath

    await invoke('delete_object', {
      endpoint_url: endpointUrl,
      bucket,
      s3Key,
    })

    // 删除成功后，从远程集合中移除
    remotePaths.delete(relativePath)
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
 * 2. 扫描本地文件更新响应式集合
 * 3. 获取远程文件更新响应式集合
 * 4. 上传本地文件
 * 5. 可选：删除远程多余文件
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

  try {
    const remotePrefix = (`${form.remote_dir}/`).replace(/^[\\/]+/, '').replace(/[\\/]+/g, '/')

    // 1. 清空响应式集合
    localPaths.clear()
    remotePaths.clear()

    // 2. 获取本地文件路径列表并追加到响应式集合
    await updateLocalFilePaths(form.local_dir)

    // 3. 获取远程文件路径列表并追加到响应式集合
    await updateRemoteFilePaths(form.endpoint_url, form.bucket, remotePrefix)

    // 4. 检查是否需要操作
    if (localPaths.size === 0 && remotePaths.size === 0) {
      ElMessage.success('本地和远程都没有文件，无需操作')
      return
    }

    if (localPaths.size === 0 && !form.delete_remote_extras) {
      ElMessage.info('本地没有文件，且未启用删除远程文件，无需操作')
      return
    }

    // 5. 上传本地文件
    if (localPaths.size > 0) {
      await uploadLocalFiles(form.local_dir, form.endpoint_url, form.bucket, remotePrefix)
    }

    // 6. 删除远程多余文件（如果启用）
    if (form.delete_remote_extras && remotePaths.size > 0) {
      await deleteRemoteExtraFiles(form.endpoint_url, form.bucket, remotePrefix)
    }

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

      <ElFormItem prop="delete_remote_extras">
        <ElCheckbox v-model="form.delete_remote_extras">
          删除远程多余文件
        </ElCheckbox>
      </ElFormItem>

      <div v-if="!loading" class="mt-4">
        <ElButton type="primary" native-type="submit">
          <CloudUpload :size="18" class="mr-2" />
          开始上传
        </ElButton>
      </div>
    </ElForm>
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
