<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { Store } from "@tauri-apps/plugin-store";
import { merge } from "lodash-es";
import { CloudDownload, CloudUpload, Database, Folder } from "lucide-vue-next";
import {
  boolean,
  literal,
  object,
  string,
  union,
  type infer as Infer,
} from "zod/mini";
import type { FileInfo } from "@/pages/file-copy/components/file-operations";
import S3InstanceSelector from "./components/S3InstanceSelector.vue";
import type { S3Object } from "./components/s3-files";
import type { FormRules } from "element-plus";

/**
 * 保存的表单数据 Zod 模式定义
 */
const FormDataZod = object({
  /** 存储桶名称 */
  bucket: string(),
  /** S3 实例唯一标识符 */
  s3_instance_id: string(),
  /** 本地目录路径 */
  local_dir: string(),
  /** 远程目录路径 */
  remote_dir: string(),
  /** 同步方向 */
  sync_direction: union([
    literal("local-to-remote"),
    literal("remote-to-local"),
  ]),
  /** 是否删除多余文件 */
  delete_extras: boolean(),
});

/** 表单数据类型 */
interface FormData extends Infer<typeof FormDataZod> {}

// 表单数据
const form = reactive<Partial<FormData>>({
  // 默认本地到远程
  sync_direction: "local-to-remote",
  // 默认勾选删除多余文件
  delete_extras: true,
});

// 表单引用
const formRef = useTemplateRef("form-ref");

// 校验规则
const rules = reactive<FormRules>({
  s3_instance_id: [{ required: true, message: "请选择 S3 实例" }],
  bucket: [{ required: true, message: "请输入存储桶名称" }],
  local_dir: [{ required: true, message: "请选择本地目录" }],
  remote_dir: [{ required: true, message: "请输入远程目录路径" }],
});

const loading = ref(false);

// 响应式路径集合
const localPaths = reactive<Set<string>>(new Set());
const remotePaths = reactive<Set<string>>(new Set());

const FORM_STORAGE_KEY = "s3-sync-form";
const store = Store.load("form-data.json");

/**
 * 初始化时加载保存的表单数据
 */
store.then(async (store) => {
  const data = await store.get(FORM_STORAGE_KEY);
  const result = FormDataZod.safeParse(data);
  if (!result.success) {
    return;
  }
  merge(form, result.data);
  // 数据回显后移除表单校验状态
  await nextTick();
  formRef.value?.clearValidate();
});

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
    const files = await invoke<FileInfo[]>("list_directory", { path: dirPath });

    for (const file of files) {
      if (file.is_dir) {
        // 递归处理子目录
        await scanDirectory(file.path);
      } else {
        // 移除开头的路径分隔符并统一为正斜杠
        const relativePath = file.path
          .replace(dir, "")
          .replace(/^[\\/]+/, "")
          .replaceAll(/[\\/]+/g, "/");

        localPaths.add(relativePath);
      }
    }
  }

  await scanDirectory(dir);
}

/**
 * 获取远程文件路径列表
 *
 * 递归获取远程S3存储桶中的所有文件，追加到响应式集合。
 *
 * @param s3InstanceId - S3 实例的唯一标识符
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 */
async function updateRemoteFilePaths(
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  let continuationToken: string | undefined;

  do {
    // 调用后端API获取一页对象
    const response = await invoke<{
      objects: S3Object[];
      is_truncated: boolean;
      next_continuation_token?: string;
    }>("list_s3_objects", {
      s3_instance_id: s3InstanceId,
      bucket,
      prefix,
      continuation_token: continuationToken,
    });

    // 转换为相对路径并添加到响应式集合
    for (const obj of response.objects) {
      const relativePath = obj.key.replace(prefix, "").replace(/^\/+/, ""); // 移除开头的斜杠（S3路径只使用正斜杠）

      if (relativePath) {
        remotePaths.add(relativePath);
      }
    }

    // 检查是否还有更多数据
    continuationToken = response.next_continuation_token;

    // 如果还有更多数据，添加短暂延迟
    if (continuationToken) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  } while (continuationToken);
}

/**
 * 上传本地文件到S3
 *
 * 遍历本地文件路径列表，上传文件到S3存储桶
 *
 * @param localDir - 本地目录路径
 * @param s3InstanceId - S3 实例的唯一标识符
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 * @throws {Error} 当上传失败时抛出错误
 */
async function uploadLocalFiles(
  localDir: string,
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  // 使用数组来遍历，避免在迭代过程中修改集合
  const pathsToUpload = Array.from(localPaths);

  for (const relativePath of pathsToUpload) {
    // 使用Tauri的join函数进行跨平台路径规范化拼接
    const localPath = await join(localDir, relativePath);
    const s3Key = prefix + relativePath;

    await invoke("upload_file_to_s3", {
      s3_instance_id: s3InstanceId,
      bucket,
      localPath,
      s3Key,
    });

    // 上传成功后，从本地集合中删除
    localPaths.delete(relativePath);
    // 同时从远程集合中删除（如果存在的话）
    remotePaths.delete(relativePath);
  }
}

/**
 * 下载远程文件到本地
 *
 * 遍历远程文件路径列表，下载文件到本地目录
 *
 * @param localDir - 本地目录路径
 * @param s3InstanceId - S3 实例的唯一标识符
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 * @throws {Error} 当下载失败时抛出错误
 */
async function downloadRemoteFiles(
  localDir: string,
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  // 使用数组来遍历，避免在迭代过程中修改集合
  const pathsToDownload = Array.from(remotePaths);

  for (const relativePath of pathsToDownload) {
    const localPath = await join(localDir, relativePath);
    const s3Key = prefix + relativePath;

    await invoke("download_file_from_s3", {
      s3_instance_id: s3InstanceId,
      bucket,
      localPath,
      s3Key,
    });

    // 下载成功后，从远程集合中删除
    remotePaths.delete(relativePath);
    // 同时从本地集合中删除（如果存在的话）
    localPaths.delete(relativePath);
  }
}

/**
 * 删除本地多余文件
 *
 * 删除本地集合中剩余的所有文件（这些文件在远程不存在）
 *
 * @param localDir - 本地目录路径
 * @throws {Error} 当删除失败时抛出错误
 */
async function deleteLocalExtraFiles(localDir: string): Promise<void> {
  // 下载完成后，localPaths中剩下的就是需要删除的本地多余文件
  // 直接遍历并删除所有剩余文件
  const filesToDelete = Array.from(localPaths);

  for (const relativePath of filesToDelete) {
    const localPath = await join(localDir, relativePath);

    await invoke("remove_path", {
      path: localPath,
    });

    // 删除成功后，从本地集合中移除
    localPaths.delete(relativePath);
  }
}

/**
 * 删除远程多余文件
 *
 * 删除远程集合中剩余的所有文件（这些文件在本地不存在）
 *
 * @param s3InstanceId - S3 实例的唯一标识符
 * @param bucket - 存储桶名称
 * @param prefix - S3 对象键前缀
 * @throws {Error} 当删除失败时抛出错误
 */
async function deleteRemoteExtraFiles(
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  // 上传完成后，remotePaths中剩下的就是需要删除的多余文件
  // 直接遍历并删除所有剩余文件
  const filesToDelete = Array.from(remotePaths);

  for (const relativePath of filesToDelete) {
    const s3Key = prefix + relativePath;

    await invoke("delete_s3_object", {
      s3_instance_id: s3InstanceId,
      bucket,
      s3Key,
    });

    // 删除成功后，从远程集合中移除
    remotePaths.delete(relativePath);
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
  });
  if (!selected) return;
  form.local_dir = selected;
}

/**
 * 开始同步流程
 *
 * 执行完整的 S3 同步流程：
 * 1. 验证表单并保存配置
 * 2. 扫描本地和远程文件更新响应式集合
 * 3. 根据同步方向执行相应的同步操作
 *
 * @throws {Error} 当同步过程中出现错误时抛出异常
 */
async function startSync() {
  await formRef.value?.validate();

  const formData = FormDataZod.parse(form);

  // 保存表单数据（包含所有信息）
  await store.then(async (store) => {
    await store.set(FORM_STORAGE_KEY, formData);
    await store.save();
  });

  loading.value = true;

  try {
    const remotePrefix = `${formData.remote_dir}/`
      .replace(/^[\\/]+/, "")
      .replaceAll(/[\\/]+/g, "/");

    // 1. 清空响应式集合
    localPaths.clear();
    remotePaths.clear();

    // 2. 获取本地和远程文件路径列表
    await updateLocalFilePaths(formData.local_dir);
    await updateRemoteFilePaths(
      formData.s3_instance_id,
      formData.bucket,
      remotePrefix,
    );

    // 3. 根据同步方向执行不同的逻辑
    switch (formData.sync_direction) {
      case "local-to-remote":
        // 本地到远程同步
        if (localPaths.size === 0 && remotePaths.size === 0) {
          ElMessage.success("本地和远程都没有文件，无需操作");
          return;
        }

        if (localPaths.size === 0 && !formData.delete_extras) {
          ElMessage.info("本地没有文件，且未启用删除远程文件，无需操作");
          return;
        }

        // 上传本地文件
        if (localPaths.size > 0) {
          await uploadLocalFiles(
            formData.local_dir,
            formData.s3_instance_id,
            formData.bucket,
            remotePrefix,
          );
        }

        // 删除远程多余文件（如果启用）
        if (formData.delete_extras && remotePaths.size > 0) {
          await deleteRemoteExtraFiles(
            formData.s3_instance_id,
            formData.bucket,
            remotePrefix,
          );
        }

        ElMessage.success("S3 同步完成（本地 → 远程）");
        break;

      case "remote-to-local":
        // 远程到本地同步
        if (localPaths.size === 0 && remotePaths.size === 0) {
          ElMessage.success("本地和远程都没有文件，无需操作");
          return;
        }

        if (remotePaths.size === 0 && !formData.delete_extras) {
          ElMessage.info("远程没有文件，且未启用删除本地文件，无需操作");
          return;
        }

        // 下载远程文件
        if (remotePaths.size > 0) {
          await downloadRemoteFiles(
            formData.local_dir,
            formData.s3_instance_id,
            formData.bucket,
            remotePrefix,
          );
        }

        // 删除本地多余文件（如果启用）
        if (formData.delete_extras && localPaths.size > 0) {
          await deleteLocalExtraFiles(formData.local_dir);
        }

        ElMessage.success("S3 同步完成（远程 → 本地）");
        break;
    }
  } catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(`S3 同步失败: ${errorMsg}`);
  } finally {
    loading.value = false;
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
      @submit.prevent="startSync"
    >
      <ElFormItem label="S3 实例" prop="s3_instance_id">
        <S3InstanceSelector
          v-model="form.s3_instance_id"
          :class="$style.instanceSelector"
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

      <ElFormItem label="同步方向" prop="sync_direction">
        <ElRadioGroup v-model="form.sync_direction">
          <ElRadio value="local-to-remote"> 本地 → 远程 </ElRadio>
          <ElRadio value="remote-to-local"> 远程 → 本地 </ElRadio>
        </ElRadioGroup>
      </ElFormItem>

      <ElFormItem label="本地目录" prop="local_dir">
        <ElInput
          v-model="form.local_dir"
          placeholder="点击选择要上传的本地目录..."
          :class="$style.pathInput"
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
          :class="$style.pathInput"
          :disabled="loading"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem prop="delete_extras">
        <ElCheckbox v-model="form.delete_extras"> 删除多余文件 </ElCheckbox>
      </ElFormItem>

      <div v-if="!loading" class="mt-4">
        <ElButton type="primary" native-type="submit">
          <template v-if="form.sync_direction === 'local-to-remote'">
            <CloudUpload :size="18" class="mr-2" />
            开始同步（本地 → 远程）
          </template>
          <template v-else-if="form.sync_direction === 'remote-to-local'">
            <CloudDownload :size="18" class="mr-2" />
            开始同步（远程 → 本地）
          </template>
        </ElButton>
      </div>
    </ElForm>

    <ElRow v-if="localPaths.size > 0 || remotePaths.size > 0" class="my-6">
      <ElCol :span="12">
        <ElStatistic
          class="text-center"
          title="本地文件数量"
          :value="localPaths.size"
        />
      </ElCol>
      <ElCol :span="12">
        <ElStatistic
          class="text-center"
          title="远程文件数量"
          :value="remotePaths.size"
        />
      </ElCol>
    </ElRow>
  </div>
</template>

<style module>
.instanceSelector {
  width: 25rem;
}

.pathInput {
  width: 30rem;
}

.bucketInput {
  width: 20rem;
}
</style>
