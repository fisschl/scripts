<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import {
  ArrowLeft,
  File,
  Folder,
  FolderOpen,
  RefreshCw,
} from "lucide-vue-next";
import prettyBytes from "pretty-bytes";
import { useAsyncLoad } from "@/utils/loader";
import type { FileInfo } from "@/pages/file-copy/components/file-operations";

/**
 * 路由相关
 */
const route = useRoute();
const router = useRouter();

/**
 * 当前目录路径
 */
const currentPath = ref<string>("");

/**
 * 计算面包屑路径列表
 */
const breadcrumbPaths = computed(() => {
  const segments = currentPath.value.split(/[\\/]+/);
  return segments.map((segment, index) => {
    const path = segments.slice(0, index + 1).join("\\");
    return {
      name: segment || "根目录",
      path,
      to: `/file-manager?${new URLSearchParams({ path })}`,
    };
  });
});

/**
 * 监听URL路径变化
 */
watch(
  () => route.query.path,
  (newPath) => {
    if (typeof newPath === "string" && newPath !== currentPath.value) {
      currentPath.value = newPath;
    }
  },
  { immediate: true },
);

/**
 * 更新URL参数
 */
function updateUrlPath(path: string) {
  router.replace({
    query: { path },
  });
}

/**
 * 当前目录路径参数，用于 useAsyncLoad
 */
const directoryPath = computed(() => currentPath.value || "");

/**
 * 使用 useAsyncLoad 加载目录内容
 */
const files = useAsyncLoad<string, FileInfo[]>({
  params: directoryPath,
  fetcher: async (path: string) => {
    if (!path) return [];
    const directoryFiles = await invoke<FileInfo[]>("list_directory", { path });
    return directoryFiles.toSorted((a, b) => {
      // 目录优先排序，然后按名称排序
      if (a.is_dir && !b.is_dir) return -1;
      if (!a.is_dir && b.is_dir) return 1;
      return a.path.localeCompare(b.path);
    });
  },
});

/**
 * 组件挂载时加载用户主目录
 */
onMounted(async () => {
  await loadHomeDirectory();
});

/**
 * 加载用户主目录
 */
async function loadHomeDirectory() {
  try {
    const homePath = await homeDir();
    if (homePath) {
      currentPath.value = homePath;
      updateUrlPath(homePath);
    } else {
      throw new Error("无法获取用户主目录");
    }
  } catch (error) {
    ElMessage.error(`获取用户主目录失败: ${error}`);
  }
}

/**
 * 刷新当前目录
 */
async function refreshDirectory() {
  if (!currentPath.value) {
    await loadHomeDirectory();
  }
}

/**
 * 处理目录点击事件
 */
function handleDirectoryClick(fileInfo: FileInfo) {
  if (fileInfo.is_dir) {
    currentPath.value = fileInfo.path;
    updateUrlPath(fileInfo.path);
  }
}

/**
 * 返回上级目录
 */
function goBack() {
  const parentPath = currentPath.value
    .split(/[\\/]+/)
    .slice(0, -1)
    .join("\\");
  if (parentPath) {
    currentPath.value = parentPath;
    updateUrlPath(parentPath);
  }
}

/**
 * 格式化修改时间
 */
function formatModifiedTime(timeString: string): string {
  return new Date(timeString).toLocaleString("zh-CN");
}

/**
 * 获取文件名
 */
function getFileName(path: string): string {
  return path.split(/[\\/]+/).pop() || path;
}

/**
 * 判断是否可以返回上级目录
 */
const canGoBack = computed(() => {
  return (
    currentPath.value !== "" && currentPath.value.split(/[\\/]+/).length > 1
  );
});
</script>

<template>
  <div
    class="flex h-full flex-col border-b border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800"
  >
    <div class="flex items-center">
      <ElButton
        :disabled="!canGoBack"
        class="flex items-center"
        @click="goBack"
      >
        <ArrowLeft :size="16" class="mr-1" />
      </ElButton>
      <ElBreadcrumb class="flex-1">
        <ElBreadcrumbItem
          v-for="(item, index) in breadcrumbPaths"
          :key="index"
          :to="item.to"
        >
          {{ item.name }}
        </ElBreadcrumbItem>
      </ElBreadcrumb>
      <ElButton @click="refreshDirectory">
        <RefreshCw :size="16" class="mr-1" />
      </ElButton>
    </div>

    <ElTable :data="files" class="h-full flex-1 overflow-auto p-4" stripe>
      <ElTableColumn min-width="300">
        <template #header>
          <div class="flex items-center">
            <Folder :size="16" class="mr-2" />
            名称
          </div>
        </template>
        <template #default="{ row }">
          <div
            class="flex cursor-pointer items-center hover:text-blue-500"
            @click="handleDirectoryClick(row)"
          >
            <FolderOpen
              v-if="row.is_dir"
              :size="16"
              class="mr-2 text-blue-500"
            />
            <File v-else :size="16" class="mr-2 text-gray-500" />
            <span class="font-mono">{{ getFileName(row.path) }}</span>
          </div>
        </template>
      </ElTableColumn>

      <ElTableColumn prop="size" label="大小" width="120" align="right">
        <template #default="{ row }">
          <span class="font-mono text-sm">
            {{ row.is_dir ? "-" : prettyBytes(row.size) }}
          </span>
        </template>
      </ElTableColumn>

      <ElTableColumn
        prop="last_modified"
        label="修改时间"
        width="180"
        align="center"
      >
        <template #default="{ row }">
          <span class="text-sm text-gray-600 dark:text-gray-400">
            {{ formatModifiedTime(row.last_modified) }}
          </span>
        </template>
      </ElTableColumn>

      <ElTableColumn type="expand" width="50">
        <template #default="{ row }">
          <div class="bg-gray-50 p-4 dark:bg-gray-900">
            <div class="grid grid-cols-1 gap-2 font-mono text-sm">
              <div>
                <span class="font-semibold">完整路径:</span>
                <span class="ml-2">{{ row.path }}</span>
              </div>
              <div>
                <span class="font-semibold">类型:</span>
                <span class="ml-2">{{ row.is_dir ? "目录" : "文件" }}</span>
              </div>
              <div v-if="!row.is_dir">
                <span class="font-semibold">文件大小:</span>
                <span class="ml-2">
                  {{ prettyBytes(row.size) }} ({{ row.size }} 字节)
                </span>
              </div>
            </div>
          </div>
        </template>
      </ElTableColumn>
    </ElTable>
  </div>
</template>
