<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge } from 'lodash-es'
import { Copy, Folder } from 'lucide-vue-next'
import { array, object, string } from 'zod/mini'
import LogViewer from '@/components/LogViewer.vue'

const FormDataZod = object({
  sourcePath: string(),
  targetPath: string(),
  extensions: array(string()),
})

// 表单数据
const form = reactive<Infer<typeof FormDataZod>>({
  sourcePath: '',
  targetPath: '',
  extensions: ['mp4', 'webm', 'm4v'], // 只保留浏览器原生支持的格式
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  sourcePath: [{ required: true, message: '请选择源目录', trigger: 'blur' }],
  targetPath: [{ required: true, message: '请选择目标目录', trigger: 'blur' }],
  extensions: [
    {
      type: 'array',
      required: true,
      message: '请至少选择一个文件扩展名',
      trigger: 'change',
    },
    {
      type: 'array',
      min: 1,
      message: '请至少选择一个文件扩展名',
      trigger: 'change',
    },
  ],
})

const loading = ref(false)
const progressMessages = ref<string[]>([])

const FORM_STORAGE_KEY = 'file-copy-form'
const store = Store.load('form-data.json')

// 初始化时加载保存的表单数据
store.then(async (store) => {
  const result = FormDataZod.safeParse(await store.get(FORM_STORAGE_KEY))
  if (!result.success)
    return
  merge(form, result.data)
})

async function selectSourcePath() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.sourcePath = selected
}

async function selectTargetPath() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.targetPath = selected
}

async function startCopy() {
  await formRef.value?.validate()

  // 保存表单数据
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, form)
    await store.save()
  })

  loading.value = true
  progressMessages.value = ['开始文件复制操作...']

  try {
    const result = await invoke('copy_files_with_options', {
      from: form.sourcePath,
      to: form.targetPath,
      extensions: form.extensions,
    })
    if (typeof result !== 'number')
      throw new Error('复制文件操作返回非数字类型')

    progressMessages.value.push(`复制完成，共复制 ${result} 个文件`)
    ElMessage.success(`文件复制完成，共复制 ${result} 个文件`)
  }
  catch (error: unknown) {
    progressMessages.value.push(`错误: ${error}`)
    ElMessage.error(`复制失败: ${error}`)
  }
  finally {
    loading.value = false
  }
}

const effects: (() => unknown)[] = []

onMounted(async () => {
  effects.push(
    await listen('file-copy-progress', (event) => {
      const { payload } = event
      if (typeof payload !== 'string')
        return
      progressMessages.value.push(`正在复制: ${payload}`)
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
      @submit.prevent="startCopy"
    >
      <ElFormItem label="源目录" prop="sourcePath">
        <ElInput
          v-model="form.sourcePath"
          placeholder="点击选择源目录..."
          :class="$style.input"
          :disabled="loading"
          @click="selectSourcePath"
        >
          <template #prefix>
            <ElIcon>
              <Folder />
            </ElIcon>
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="目标目录" prop="targetPath">
        <ElInput
          v-model="form.targetPath"
          placeholder="点击选择目标目录..."
          :class="$style.input"
          :disabled="loading"
          @click="selectTargetPath"
        >
          <template #prefix>
            <ElIcon>
              <Folder />
            </ElIcon>
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="文件扩展名" prop="extensions">
        <ElInputTag
          v-model="form.extensions"
          placeholder="请输入扩展名"
          :disabled="loading"
        />
      </ElFormItem>

      <div class="mt-4">
        <ElButton type="primary" :loading="loading" native-type="submit">
          <Copy :size="18" class="mr-2" />
          开始复制
        </ElButton>
      </div>
    </ElForm>
    <LogViewer :logs="progressMessages" class="my-4" />
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
