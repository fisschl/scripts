<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Store } from "@tauri-apps/plugin-store";
import { merge } from "lodash-es";
import { Copy, Folder } from "lucide-vue-next";
import { array, boolean, object, string, type infer as Infer } from "zod/mini";
import {
  getFileExtension,
  type FileInfo,
} from "@/pages/file-copy/components/file-operations";
import type { FormRules } from "element-plus";

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
  /** 是否在复制后删除源文件（剪切模式） */
  moveAfterCopy: boolean(),
});

/** 表单数据类型 */
interface FormData extends Infer<typeof SavedFormDataZod> {}

// 表单数据
const form = reactive<FormData>({
  sourcePath: "",
  targetPath: "",
  extensions: ["mp4", "webm", "m4v"], // 只保留浏览器原生支持的格式
  moveAfterCopy: false, // 默认为复制模式
});

// 表单引用
const formRef = useTemplateRef("form-ref");

// 校验规则
const rules = reactive<FormRules>({
  sourcePath: [{ required: true, message: "请选择源目录" }],
  targetPath: [{ required: true, message: "请选择目标目录" }],
  extensions: [
    {
      type: "array",
      required: true,
      message: "请至少选择一个文件扩展名",
    },
    {
      type: "array",
      min: 1,
      message: "请至少选择一个文件扩展名",
    },
  ],
});

/** 表单存储键名 */
const FORM_STORAGE_KEY = "file-copy-form";
const store = Store.load("form-data.json");

/**
 * 初始化时加载保存的表单数据
 */
store.then(async (store) => {
  const result = SavedFormDataZod.safeParse(await store.get(FORM_STORAGE_KEY));
  if (!result.success) return;
  merge(form, result.data);
  // 数据回显后移除表单校验状态
  await nextTick();
  formRef.value?.clearValidate();
});

/**
 * 选择源目录
 */
async function selectSourcePath() {
  const selected = await open({
    multiple: false,
    directory: true,
  });
  if (!selected) return;
  form.sourcePath = selected;
}

/**
 * 选择目标目录
 */
async function selectTargetPath() {
  const selected = await open({
    multiple: false,
    directory: true,
  });
  if (!selected) return;
  form.targetPath = selected;
}

/** 加载状态 */
const loading = ref(false);

/** 当前处理的文件状态 */
const currentFile = ref<string>();

/**
 * 串行化扫描和复制文件
 *
 * 递归扫描目录，边扫描边复制，实现串行化处理
 */
async function scanAndCopy(
  dirPath: string,
  allowedExtensions: string[],
): Promise<void> {
  const entries = await invoke<FileInfo[]>("list_directory", { path: dirPath });

  for (const entry of entries) {
    if (entry.is_dir) {
      // 递归处理子目录
      await scanAndCopy(entry.path, allowedExtensions);
      continue;
    }
    // 检查文件扩展名
    const ext = getFileExtension(entry.path);
    if (!ext || !allowedExtensions.includes(ext)) continue;

    currentFile.value = entry.path;

    // 复制文件
    await copySingleFile(entry.path, form.targetPath);
  }
}

/**
 * 开始文件复制流程
 *
 * 执行串行化的文件复制流程：
 * 1. 边扫描边复制，递归处理目录
 * 2. 根据扩展名筛选文件
 * 3. 逐个复制文件到目标目录
 * 4. 使用哈希值生成唯一文件名避免冲突
 */
async function startCopy() {
  await formRef.value?.validate();

  // 保存表单数据
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, form);
    await store.save();
  });

  loading.value = true;
  currentFile.value = "";

  try {
    const { sourcePath, extensions } = form;
    const allowedExtensions = extensions.map((ext) => ext.toLowerCase());

    // 开始串行化扫描和复制
    await scanAndCopy(sourcePath, allowedExtensions);

    ElMessage.success(`文件${form.moveAfterCopy ? "剪切" : "复制"}完成！`);
    currentFile.value = "";
  } catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(
      `文件${form.moveAfterCopy ? "剪切" : "复制"}失败: ${errorMsg}`,
    );
  } finally {
    loading.value = false;
  }
}

const loadingText = computed(() => {
  if (!currentFile.value) return;
  return currentFile.value.replace(form.sourcePath, "");
});

/**
 * 复制单个文件 - 使用通用的 calculate_file_hash 指令
 *
 * 复制后的文件使用 Blake3 哈希算法生成唯一文件名，保留原始扩展名
 * 如果目标目录已存在同名文件，则跳过该文件
 * 如果启用了剪切模式，复制成功后删除源文件
 */
async function copySingleFile(
  sourceFile: string,
  targetDir: string,
): Promise<void> {
  // 1. 调用通用哈希指令计算文件哈希值
  const hash = await invoke<string>("file_hash", {
    filePath: sourceFile,
  });

  // 2. 获取文件扩展名
  const extension = getFileExtension(sourceFile) || undefined;

  // 3. 构建目标文件路径（哈希值 + 原始扩展名）
  const targetFileName = extension ? `${hash}.${extension}` : hash;
  const targetPath = `${targetDir}/${targetFileName}`;

  // 4. 执行文件复制（如果不允许覆盖，则跳过已存在的文件）
  await invoke("copy_file", {
    from: sourceFile,
    to: targetPath,
    overwrite: false,
  });

  // 5. 如果启用了剪切模式，复制成功后删除源文件
  if (form.moveAfterCopy) {
    await invoke("remove_path", {
      path: sourceFile,
    });
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

      <ElFormItem prop="moveAfterCopy">
        <ElCheckbox v-model="form.moveAfterCopy">
          复制后删除源文件（剪切）
        </ElCheckbox>
      </ElFormItem>

      <div v-if="!loading" class="mt-4">
        <ElButton type="primary" native-type="submit">
          <Copy :size="18" class="mr-2" />
          开始复制
        </ElButton>
      </div>
    </ElForm>

    <!-- 当前文件 -->
    <p v-if="loadingText" class="my-6 text-gray-600 dark:text-gray-400">
      当前处理文件：
      {{ loadingText }}
    </p>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
