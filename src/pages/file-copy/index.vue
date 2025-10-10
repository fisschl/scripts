<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import type { FileInfo, FileItem } from './components/types'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge, pull } from 'lodash-es'
import { Copy, Folder } from 'lucide-vue-next'
import { array, object, string } from 'zod/mini'
import FileListItem from './components/FileListItem.vue'

const SavedFormDataZod = object({
  sourcePath: string(),
  targetPath: string(),
  extensions: array(string()),
})

interface FormData extends Infer<typeof SavedFormDataZod> {}

// 表单数据
const form = reactive<FormData>({
  sourcePath: '',
  targetPath: '',
  extensions: ['mp4', 'webm', 'm4v'], // 只保留浏览器原生支持的格式
})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 校验规则
const rules = reactive<FormRules>({
  sourcePath: [{ required: true, message: '请选择源目录' }],
  targetPath: [{ required: true, message: '请选择目标目录' }],
  extensions: [
    {
      type: 'array',
      required: true,
      message: '请至少选择一个文件扩展名',
    },
    {
      type: 'array',
      min: 1,
      message: '请至少选择一个文件扩展名',
    },
  ],
})

const showProgressDrawer = ref(false)

const FORM_STORAGE_KEY = 'file-copy-form'
const store = Store.load('form-data.json')

// 初始化时加载保存的表单数据
store.then(async (store) => {
  const result = SavedFormDataZod.safeParse(await store.get(FORM_STORAGE_KEY))
  if (!result.success)
    return
  merge(form, result.data)
  // 数据回显后移除表单校验状态
  await nextTick()
  formRef.value?.clearValidate()
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

const loading = ref(false)

const files = reactive<FileItem[]>([])

async function startCopy() {
  await formRef.value?.validate()

  // 保存表单数据
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, form)
    await store.save()
  })

  showProgressDrawer.value = true
  loading.value = true

  try {
    const { sourcePath, extensions } = form

    // 1. 递归列举源目录中的所有文件
    const allFiles = await listFilesRecursive(sourcePath)
    if (allFiles.length === 0) {
      throw new Error('源目录为空或无法访问')
    }

    files.length = 0
    // 2. 筛选匹配扩展名的文件
    const allowedExtensions = extensions.map(ext => ext.toLowerCase())
    const filteredFiles = allFiles.filter(file =>
      allowedExtensions.some(ext => file.toLowerCase().endsWith(`.${ext}`)),
    )

    filteredFiles.forEach((filePath) => {
      files.push({ path: filePath, status: 'pending' })
    })

    for (const item of files) {
      item.status = 'processing'
      await copySingleFile(item.path, form.targetPath)
      pull(files, item)
    }
  }
  catch (error: unknown) {
    ElMessage.error(`复制失败: ${error}`)
  }
  finally {
    loading.value = false
    showProgressDrawer.value = false
  }
}

/**
 * 递归列举目录中的所有文件
 */
async function listFilesRecursive(dirPath: string): Promise<string[]> {
  const result: string[] = []

  const files = await invoke<FileInfo[]>('list_directory', {
    args: {
      path: dirPath,
    },
  })

  for (const file of files) {
    if (file.is_dir) {
      // 递归处理子目录
      const subFiles = await listFilesRecursive(file.path)
      result.push(...subFiles)
    }
    else {
      // 添加文件路径
      result.push(file.path)
    }
  }

  return result
}

/**
 * 复制单个文件 - 使用通用的 calculate_file_hash 指令
 *
 * 复制后的文件使用 Blake3 哈希算法生成唯一文件名，保留原始扩展名
 * 如果目标目录已存在同名文件，则跳过该文件
 */
async function copySingleFile(
  sourceFile: string,
  targetDir: string,
): Promise<void> {
  // 1. 调用通用哈希指令计算文件哈希值
  const hash = await invoke<string>('calculate_file_hash', {
    filePath: sourceFile,
  })

  // 2. 获取文件扩展名
  const lastDotIndex = sourceFile.lastIndexOf('.')
  const extension = lastDotIndex > -1 ? sourceFile.substring(lastDotIndex + 1) : ''

  // 3. 构建目标文件路径（哈希值 + 原始扩展名）
  const targetFileName = extension ? `${hash}.${extension}` : hash
  const targetPath = `${targetDir}/${targetFileName}`

  // 4. 执行文件复制（如果不允许覆盖，则跳过已存在的文件）
  await invoke('copy_file', {
    args: {
      from: sourceFile,
      to: targetPath,
      overwrite: false,
    },
  })
}
</script>

<template>
  <div class="p-4">
    <ElForm
      ref="form-ref"
      :model="form"
      :rules="rules"
      label-position="top"
      :disabled="loading"
      label-suffix="："
      @submit.prevent="startCopy"
    >
      <ElFormItem label="源目录" prop="sourcePath">
        <ElInput
          v-model="form.sourcePath"
          placeholder="点击选择源目录..."
          :class="$style.input"
          @click="selectSourcePath"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="目标目录" prop="targetPath">
        <ElInput
          v-model="form.targetPath"
          placeholder="点击选择目标目录..."
          :class="$style.input"
          @click="selectTargetPath"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="文件扩展名" prop="extensions">
        <ElInputTag
          v-model="form.extensions"
          placeholder="请输入扩展名"
          @keydown.enter.prevent
        />
      </ElFormItem>

      <div class="mt-4">
        <ElButton type="primary" native-type="submit" :loading="loading">
          <Copy :size="18" class="mr-2" />
          开始复制
        </ElButton>
      </div>
    </ElForm>

    <!-- 文件复制进度抽屉 -->
    <ElDrawer
      v-model="showProgressDrawer"
      :title="`待处理文件（${files.length}）`"
      direction="rtl"
      size="400"
    >
      <ul>
        <FileListItem
          v-for="(file, index) in files"
          :key="index"
          :file="file"
        />
      </ul>
    </ElDrawer>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
