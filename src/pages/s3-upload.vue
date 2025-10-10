<script setup lang="ts">
import type { FormRules } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge } from 'lodash-es'
import { CloudUpload, Database, Folder, HardDrive, MapPin } from 'lucide-vue-next'
import LogViewer from '@/components/LogViewer.vue'

// S3 实例类型
interface S3Instance {
  endpoint_url: string
  access_key_id: string
  secret_access_key: string
  region: string
}

// S3 配置类型
interface S3Config {
  bucket: string
  endpoint_url: string
}

// S3 对象类型
interface S3Object {
  key: string
  size?: number
  last_modified?: string
  etag?: string
}

// 表单数据类型
interface FormData {
  s3_instance: S3Instance
  s3_config: S3Config
  local_dir: string
  remote_dir: string
}

// 本地文件信息结构
interface LocalFile {
  path: string
  size: number
  lastModified: Date
}

// 同步操作类型
interface SyncOperation {
  type: 'upload' | 'delete'
  localPath?: string
  s3Key: string
}

// S3 实例列表
const s3Instances = ref<S3Instance[]>([])

// 表单数据
const form = reactive<FormData>({
  s3_instance: {
    endpoint_url: '',
    access_key_id: '',
    secret_access_key: '',
    region: '',
  },
  s3_config: {
    bucket: '',
    endpoint_url: '',
  },
  local_dir: '',
  remote_dir: '',
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  's3_instance.endpoint_url': [
    { required: true, message: '请选择 S3 实例' },
  ],
  's3_config.bucket': [
    { required: true, message: '请输入存储桶名称' },
  ],
  's3_config.endpoint_url': [
    { required: true, message: '请输入终端节点 URL' },
  ],
  'local_dir': [
    { required: true, message: '请选择本地目录' },
  ],
  'remote_dir': [
    { required: true, message: '请输入远程目录路径' },
  ],
})

const loading = ref(false)
const progressMessages = ref<string[]>([])

const S3_INSTANCES_KEY = 's3-instances'
const FORM_STORAGE_KEY = 's3-upload-form'
const s3Store = Store.load('s3-config.json')
const store = Store.load('form-data.json')

// 加载 S3 实例列表
async function loadS3Instances() {
  const data = await s3Store.then(store => store.get(S3_INSTANCES_KEY))
  if (Array.isArray(data)) {
    s3Instances.value = data as S3Instance[]
  }
  else {
    s3Instances.value = []
  }
}

store.then(async (store) => {
  const data = await store.get(FORM_STORAGE_KEY)
  if (data && typeof data === 'object') {
    merge(form, data)
  }
  // 数据回显后移除表单校验状态
  await nextTick()
  formRef.value?.clearValidate()
})

// 加载 S3 实例
loadS3Instances()

// 监听 S3 实例选择变化
watch(() => form.s3_instance.endpoint_url, (newEndpoint) => {
  if (newEndpoint) {
    const instance = s3Instances.value.find(item => item.endpoint_url === newEndpoint)
    if (instance) {
      form.s3_instance = { ...instance }
      form.s3_config.endpoint_url = instance.endpoint_url
    }
  }
})

// 获取本地文件列表
async function getLocalFiles(dir: string): Promise<Map<string, LocalFile>> {
  const localFiles = new Map<string, LocalFile>()

  try {
    const result = await invoke<string[]>('list_directory', { path: dir })

    for (const filePath of result) {
      try {
        const stats = await invoke<any>('get_file_stats', { path: filePath })
        if (stats.isFile) {
          const relativePath = filePath.replace(dir + (dir.endsWith('/') || dir.endsWith('\\') ? '' : '/'), '').replace(/\\/g, '/')
          localFiles.set(relativePath, {
            path: filePath,
            size: stats.size,
            lastModified: new Date(stats.lastModified),
          })
        }
      }
      catch (error) {
        console.warn(`获取文件信息失败: ${filePath}`, error)
      }
    }
  }
  catch (error) {
    throw new Error(`扫描本地目录失败: ${error}`)
  }

  return localFiles
}

// 获取远程文件列表
async function getRemoteFiles(endpointUrl: string, bucket: string, prefix: string): Promise<Map<string, S3Object>> {
  const remoteFiles = new Map<string, S3Object>()

  try {
    const objects = await invoke<S3Object[]>('list_objects', {
      endpoint_url: endpointUrl,
      bucket,
      prefix,
    })

    for (const obj of objects) {
      const relativeKey = obj.key.replace(prefix, '')
      if (relativeKey) {
        remoteFiles.set(relativeKey, obj)
      }
    }
  }
  catch (error) {
    throw new Error(`获取远程文件列表失败: ${error}`)
  }

  return remoteFiles
}

// 生成同步操作队列
function generateSyncOperations(
  localFiles: Map<string, LocalFile>,
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

// 执行同步操作
async function executeSyncOperations(
  operations: SyncOperation[],
  endpointUrl: string,
  bucket: string,
) {
  for (let i = 0; i < operations.length; i++) {
    const operation = operations[i]
    if (!operation)
      continue

    try {
      if (operation.type === 'upload' && operation.localPath) {
        progressMessages.value.push(`上传: ${operation.s3Key}`)
        await invoke('upload_file', {
          endpoint_url: endpointUrl,
          bucket,
          localPath: operation.localPath,
          s3Key: operation.s3Key,
        })
      }
      else if (operation.type === 'delete') {
        progressMessages.value.push(`删除: ${operation.s3Key}`)
        await invoke('delete_object', {
          endpoint_url: endpointUrl,
          bucket,
          s3Key: operation.s3Key,
        })
      }

      progressMessages.value.push(`完成 ${i + 1}/${operations.length}`)
    }
    catch (error) {
      const errorMsg = `${operation.type === 'upload' ? '上传' : '删除'}失败: ${operation.s3Key} - ${error}`
      progressMessages.value.push(errorMsg)
      throw new Error(errorMsg)
    }
  }
}

async function selectLocalDir() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.local_dir = selected
}

async function startUpload() {
  await formRef.value?.validate()

  // 保存表单数据（包含所有信息）
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, form)
    await store.save()
  })

  loading.value = true
  progressMessages.value = ['开始 S3 上传操作...']

  try {
    const remotePrefix = form.remote_dir.replace(/^\//, '') + (form.remote_dir.endsWith('/') ? '' : '/')

    // 1. 获取本地文件列表
    progressMessages.value.push('扫描本地文件...')
    const localFiles = await getLocalFiles(form.local_dir)
    progressMessages.value.push(`发现本地文件: ${localFiles.size} 个`)

    // 2. 获取远程文件列表
    progressMessages.value.push('获取远程文件列表...')
    const remoteFiles = await getRemoteFiles(form.s3_instance.endpoint_url, form.s3_config.bucket, remotePrefix)
    progressMessages.value.push(`发现远程文件: ${remoteFiles.size} 个`)

    // 3. 生成同步操作队列
    progressMessages.value.push('生成同步操作队列...')
    const operations = generateSyncOperations(localFiles, remoteFiles, remotePrefix)
    progressMessages.value.push(`生成操作队列: ${operations.length} 个操作`)

    if (operations.length === 0) {
      progressMessages.value.push('本地和远程文件完全一致，无需同步')
      ElMessage.success('本地和远程文件完全一致，无需同步')
      return
    }

    // 4. 执行同步操作
    progressMessages.value.push('开始执行同步操作...')
    await executeSyncOperations(operations, form.s3_instance.endpoint_url, form.s3_config.bucket)

    progressMessages.value.push('同步完成！')
    ElMessage.success('S3 上传完成')
  }
  catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error)
    progressMessages.value.push(`错误: ${errorMsg}`)
    ElMessage.error(`S3 上传失败: ${errorMsg}`)
  }
  finally {
    loading.value = false
  }
}

const effects: (() => unknown)[] = []

onMounted(async () => {
  effects.push(
    await listen('s3-sync-progress', (event) => {
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
  <div class="p-4">
    <ElForm
      ref="form-ref"
      :model="form"
      :rules="rules"
      label-position="top"
      label-suffix="："
      @submit.prevent="startUpload"
    >
      <ElFormItem label="S3 实例" prop="s3_instance.endpoint_url">
        <ElSelect
          v-model="form.s3_instance.endpoint_url"
          placeholder="请选择已配置的 S3 实例"
          :class="$style.input"
          :disabled="loading"
          filterable
        >
          <ElOption
            v-for="instance in s3Instances"
            :key="instance.endpoint_url"
            :label="instance.endpoint_url"
            :value="instance.endpoint_url"
          >
            <div class="flex items-center">
              <HardDrive :size="16" class="mr-2 text-gray-500" />
              <span>{{ instance.endpoint_url }}</span>
            </div>
          </ElOption>
        </ElSelect>
      </ElFormItem>

      <ElFormItem label="区域 (Region)" prop="s3_instance.region">
        <ElInput
          v-model.trim="form.s3_instance.region"
          placeholder="自动从选择的实例获取"
          :class="$style.regionInput"
          readonly
        >
          <template #prefix>
            <MapPin :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="存储桶名称 (Bucket)" prop="s3_config.bucket">
        <ElInput
          v-model.trim="form.s3_config.bucket"
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

      <div class="mt-4">
        <ElButton type="primary" :loading="loading" native-type="submit">
          <CloudUpload :size="18" class="mr-2" />
          开始上传
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

.bucketInput {
  max-width: 20rem;
}

.regionInput {
  max-width: 20rem;
}
</style>
