<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import type { FileItem } from './components/types'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Store } from '@tauri-apps/plugin-store'
import { merge, pull } from 'lodash-es'
import { Copy, Folder } from 'lucide-vue-next'
import { array, object, string } from 'zod/mini'
import { listFilesRecursive } from '@/pages/file-copy/components/file-operations'
import FileListItem from './components/FileListItem.vue'

/**
 * 保存的表单数据 Zod 模式定义
 */
const SavedFormDataZod = object({
  /** 源目录路径 */
  sourcePath: string(),
  /** 目标目录路径 */
  targetPath: string(),
  /** 文件扩展名列表 */
  extensions: array(string()),
})

/** 表单数据类型 */
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

/** 进度抽屉显示状态 */
const showProgressDrawer = ref(false)

/** 表单存储键名 */
const FORM_STORAGE_KEY = 'file-copy-form'
const store = Store.load('form-data.json')

/**
 * 初始化时加载保存的表单数据
 */
store.then(async (store) => {
  const result = SavedFormDataZod.safeParse(await store.get(FORM_STORAGE_KEY))
  if (!result.success)
    return
  merge(form, result.data)
  // 数据回显后移除表单校验状态
  await nextTick()
  formRef.value?.clearValidate()
})

/**
 * 选择源目录
 */
async function selectSourcePath() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.sourcePath = selected
}

/**
 * 选择目标目录
 */
async function selectTargetPath() {
  const selected = await open({
    multiple: false,
    directory: true,
  })
  if (!selected)
    return
  form.targetPath = selected
}

/** 加载状态 */
const loading = ref(false)

/** 待处理文件列表 */
const files = reactive<FileItem[]>([])

/**
 * 开始文件复制流程
 *
 * 执行完整的文件复制流程：
 * 1. 递归扫描源目录
 * 2. 根据扩展名筛选文件
 * 3. 逐个复制文件到目标目录
 * 4. 使用哈希值生成唯一文件名避免冲突
 */
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
    allFiles.filter((file) => {
      return allowedExtensions.some(ext => file.toLowerCase().endsWith(`.${ext}`))
    })
      .forEach((filePath) => {
        files.push({ path: filePath, status: 'pending' })
      })

    for (const item of files) {
      item.status = 'processing'
      await copySingleFile(item.path, form.targetPath)
      pull(files, item)
    }

    ElMessage.success('所有文件复制完成')
    // 等待1秒后关闭弹窗
    await new Promise(resolve => setTimeout(resolve, 1000))
    showProgressDrawer.value = false
  }
  catch (error: unknown) {
    ElMessage.error(`复制失败: ${error}`)
  }
  finally {
    loading.value = false
  }
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
  const extension = lastDotIndex > -1 ? sourceFile.substring(lastDotIndex + 1) : undefined

  // 3. 构建目标文件路径（哈希值 + 原始扩展名）
  const targetFileName = extension ? `${hash}.${extension}` : hash
  const targetPath = `${targetDir}/${targetFileName}`

  // 4. 执行文件复制（如果不允许覆盖，则跳过已存在的文件）
  await invoke('copy_file', {
    from: sourceFile,
    to: targetPath,
    overwrite: false,
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
