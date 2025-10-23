<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { join } from "@tauri-apps/api/path";
import { CloudDownload, CloudUpload } from "lucide-vue-next";
import type { FileInfo } from "./file-operations";
import type { S3Object } from "./s3-files";
import type { SyncPlan } from "./sync-plan";

const props = defineProps<{
  plan: SyncPlan;
}>();

const visible = defineModel<boolean>("visible");

// 响应式路径集合
const localPaths = reactive(new Set<string>());
const remotePaths = reactive(new Set<string>());

// 同步状态
const syncing = ref(false);

/**
 * 获取本地文件路径列表
 */
async function updateLocalFilePaths(rootDir: string, currentDir?: string): Promise<void> {
  const scanDir = currentDir || rootDir;
  const files = await invoke<FileInfo[]>("list_directory", { path: scanDir });

  const scanPromises = files.map(async (file) => {
    if (file.is_dir) {
      await updateLocalFilePaths(rootDir, file.path);
    } else {
      const relativePath = file.path
        .replace(rootDir, "")
        .replace(/^[\\/]+/, "")
        .replaceAll(/[\\/]+/g, "/");
      localPaths.add(relativePath);
    }
  });

  await Promise.all(scanPromises);
}

/**
 * 获取远程文件路径列表
 */
async function updateRemoteFilePaths(
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  let continuationToken: string | undefined;

  do {
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

    for (const obj of response.objects) {
      const relativePath = obj.key.replace(prefix, "").replace(/^\/+/, "");
      if (relativePath) {
        remotePaths.add(relativePath);
      }
    }

    continuationToken = response.next_continuation_token;

    if (continuationToken) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  } while (continuationToken);
}

/**
 * 上传本地文件到S3
 */
async function uploadLocalFiles(
  localDir: string,
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  const pathsToUpload = Array.from(localPaths);

  for (const relativePath of pathsToUpload) {
    const localPath = await join(localDir, relativePath);
    const s3Key = prefix + relativePath;

    await invoke("upload_file_to_s3", {
      s3_instance_id: s3InstanceId,
      bucket,
      localPath,
      s3Key,
    });

    localPaths.delete(relativePath);
    remotePaths.delete(relativePath);
  }
}

/**
 * 下载远程文件到本地
 */
async function downloadRemoteFiles(
  localDir: string,
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
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

    remotePaths.delete(relativePath);
    localPaths.delete(relativePath);
  }
}

/**
 * 删除本地多余文件
 */
async function deleteLocalExtraFiles(localDir: string): Promise<void> {
  const filesToDelete = Array.from(localPaths);

  for (const relativePath of filesToDelete) {
    const localPath = await join(localDir, relativePath);
    await invoke("remove_path", { path: localPath });
    localPaths.delete(relativePath);
  }
}

/**
 * 删除远程多余文件
 */
async function deleteRemoteExtraFiles(
  s3InstanceId: string,
  bucket: string,
  prefix: string,
): Promise<void> {
  const filesToDelete = Array.from(remotePaths);

  for (const relativePath of filesToDelete) {
    const s3Key = prefix + relativePath;
    await invoke("delete_s3_object", {
      s3_instance_id: s3InstanceId,
      bucket,
      s3Key,
    });
    remotePaths.delete(relativePath);
  }
}

/**
 * 执行从本地到远程的同步（上传）
 */
async function syncLocalToRemote() {
  syncing.value = true;

  try {
    const remotePrefix = `${props.plan.remote_dir}/`
      .replace(/^[\\/]+/, "")
      .replaceAll(/[\\/]+/g, "/");

    localPaths.clear();
    remotePaths.clear();

    await updateLocalFilePaths(props.plan.local_dir);
    await updateRemoteFilePaths(props.plan.s3_instance_id, props.plan.bucket, remotePrefix);

    if (localPaths.size === 0 && remotePaths.size === 0) {
      ElMessage.success("本地和远程都没有文件，无需操作");
      return;
    }

    if (localPaths.size === 0) {
      ElMessage.info("本地没有文件，仅删除远程多余文件");
    }

    if (localPaths.size > 0) {
      await uploadLocalFiles(
        props.plan.local_dir,
        props.plan.s3_instance_id,
        props.plan.bucket,
        remotePrefix,
      );
    }

    if (remotePaths.size > 0) {
      await deleteRemoteExtraFiles(props.plan.s3_instance_id, props.plan.bucket, remotePrefix);
    }

    ElMessage.success("S3 同步完成（本地 → 远程）");
  } catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(`S3 同步失败: ${errorMsg}`);
  } finally {
    syncing.value = false;
  }
}

/**
 * 执行从远程到本地的同步（下载）
 */
async function syncRemoteToLocal() {
  syncing.value = true;

  try {
    const remotePrefix = `${props.plan.remote_dir}/`
      .replace(/^[\\/]+/, "")
      .replaceAll(/[\\/]+/g, "/");

    localPaths.clear();
    remotePaths.clear();

    await updateLocalFilePaths(props.plan.local_dir);
    await updateRemoteFilePaths(props.plan.s3_instance_id, props.plan.bucket, remotePrefix);

    if (localPaths.size === 0 && remotePaths.size === 0) {
      ElMessage.success("本地和远程都没有文件，无需操作");
      return;
    }

    if (remotePaths.size === 0) {
      ElMessage.info("远程没有文件，仅删除本地多余文件");
    }

    if (remotePaths.size > 0) {
      await downloadRemoteFiles(
        props.plan.local_dir,
        props.plan.s3_instance_id,
        props.plan.bucket,
        remotePrefix,
      );
    }

    if (localPaths.size > 0) {
      await deleteLocalExtraFiles(props.plan.local_dir);
    }

    ElMessage.success("S3 同步完成（远程 → 本地）");
  } catch (error: unknown) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    ElMessage.error(`S3 同步失败: ${errorMsg}`);
  } finally {
    syncing.value = false;
  }
}
</script>

<template>
  <ElDialog
    v-model="visible"
    title="同步文件"
    width="500px"
    :close-on-click-modal="false"
    :close-on-press-escape="false"
  >
    <div class="flex justify-center">
      <ElButton type="primary" @click="syncRemoteToLocal">
        <CloudDownload :size="20" class="mr-2" />
        从远程到本地
      </ElButton>

      <ElButton type="success" @click="syncLocalToRemote">
        <CloudUpload :size="20" class="mr-2" />
        从本地到远程
      </ElButton>
    </div>

    <template #footer>
      <div class="flex justify-end">
        <ElButton :disabled="syncing" @click="visible = false">关闭</ElButton>
      </div>
    </template>
  </ElDialog>
</template>
