<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge } from 'lodash-es'
import { CloudUpload, Database, Folder, Globe, Key, MapPin } from 'lucide-vue-next'
import { object, string } from 'zod/mini'
import LogViewer from '@/components/LogViewer.vue'

const S3ConfigZod = object({
  access_key_id: string(),
  secret_access_key: string(),
  region: string(),
  bucket: string(),
  endpoint_url: string(),
})

const FormDataZod = object({
  s3_config: S3ConfigZod,
  local_dir: string(),
  remote_dir: string(),
})

// 表单数据
const form = reactive<Infer<typeof FormDataZod>>({
  s3_config: {
    access_key_id: '',
    secret_access_key: '',
    region: 'tos-s3-cn-shanghai',
    bucket: '',
    endpoint_url: 'https://tos-s3-cn-shanghai.volces.com',
  },
  local_dir: '',
  remote_dir: '',
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  's3_config.access_key_id': [
    { required: true, message: '请输入 Access Key ID', trigger: 'blur' },
  ],
  's3_config.secret_access_key': [
    { required: true, message: '请输入 Secret Access Key', trigger: 'blur' },
  ],
  's3_config.region': [
    { required: true, message: '请输入区域', trigger: 'blur' },
  ],
  's3_config.bucket': [
    { required: true, message: '请输入存储桶名称', trigger: 'blur' },
  ],
  's3_config.endpoint_url': [
    { required: true, message: '请输入终端节点 URL', trigger: 'blur' },
  ],
  'local_dir': [
    { required: true, message: '请选择本地目录', trigger: 'blur' },
  ],
  'remote_dir': [
    { required: true, message: '请输入远程目录路径', trigger: 'blur' },
  ],
})

const loading = ref(false)
const progressMessages = ref<string[]>([])

const FORM_STORAGE_KEY = 's3-upload-form'
const store = Store.load('form-data.json')

store.then(async (store) => {
  const result = FormDataZod.safeParse(await store.get(FORM_STORAGE_KEY))
  if (!result.success)
    return
  merge(form, result.data)
})

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
    const params = JSON.stringify(form)
    await invoke('upload_to_s3', { params })

    progressMessages.value.push('上传完成')
    ElMessage.success('S3 上传完成')
  }
  catch (error: unknown) {
    progressMessages.value.push(`错误: ${error}`)
    ElMessage.error(`S3 上传失败: ${error}`)
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
      <ElDivider>
        S3 配置
      </ElDivider>

      <ElFormItem label="Access Key ID" prop="s3_config.access_key_id">
        <ElInput
          v-model.trim="form.s3_config.access_key_id"
          placeholder="请输入 AWS Access Key ID"
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <Key :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="Secret Access Key" prop="s3_config.secret_access_key">
        <ElInput
          v-model.trim="form.s3_config.secret_access_key"
          placeholder="请输入 AWS Secret Access Key"
          type="password"
          show-password
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <Key :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="区域 (Region)" prop="s3_config.region">
        <ElInput
          v-model.trim="form.s3_config.region"
          placeholder="例如: tos-s3-cn-shanghai, us-east-1"
          :class="$style.regionInput"
          :disabled="loading"
        >
          <template #prefix>
            <MapPin :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="终端节点 URL (Endpoint)" prop="s3_config.endpoint_url">
        <ElInput
          v-model.trim="form.s3_config.endpoint_url"
          placeholder="例如: https://tos-s3-cn-shanghai.volces.com (Volces) 或 https://s3.amazonaws.com (AWS)"
          :class="$style.input"
          :disabled="loading"
        >
          <template #prefix>
            <Globe :size="16" />
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

      <ElDivider>
        上传设置
      </ElDivider>

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
