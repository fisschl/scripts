<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { homeDir } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";
import { Folder, FolderOpen, RefreshCw } from "lucide-vue-next";
import prettyBytes from "pretty-bytes";
import { useAsyncLoad } from "@/utils/loader";
import type { FileInfo } from "@/pages/file-manager/components/file-operations";

/**
 * 路由相关
 */
const route = useRoute();
const router = useRouter();

/**
 * 当前目录路径
 */
const currentPath = computed(() => {
  const { path } = route.query;
  return typeof path === "string" ? path : undefined;
});

/**
 * 组件挂载时加载用户主目录
 */
onMounted(async () => {
  const homePath = await homeDir();
  if (!homePath) return;
  router.push({
    query: {
      ...route.query,
      path: homePath,
    },
  });
});

const refreshKey = ref(1);

/**
 * 使用 useAsyncLoad 加载目录内容
 */
const files = useAsyncLoad({
  params: computed(() => ({
    refreshKey: refreshKey.value,
    path: currentPath.value,
  })),
  fetcher: async ({ path }) => {
    if (!path) return [];
    const directoryFiles = await invoke<FileInfo[]>("list_directory", { path });
    return directoryFiles;
  },
});

/**
 * 获取文件名
 */
function getFileName(path: string): string {
  return path.split(/[\\/]+/).pop() || path;
}

/**
 * 刷新当前目录
 */
function refreshDirectory() {
  refreshKey.value++;
}

function displaySize(row: FileInfo): string {
  if (row.is_dir) return "-";
  return prettyBytes(row.size);
}

async function selectFile() {
  const selected = await open({
    multiple: false,
    directory: true,
  });
  if (!selected) return;
  await router.push({
    query: {
      ...route.query,
      path: selected,
    },
  });
}
</script>

<template>
  <div class="flex h-full flex-col gap-2 p-4">
    <div class="flex items-center gap-2">
      <ElInput
        :model-value="currentPath"
        placeholder="点击选择目录"
        @click="selectFile"
      >
        <template #prefix>
          <Folder :size="16" />
        </template>
      </ElInput>
      <ElButton @click="refreshDirectory">
        <RefreshCw :size="16" />
      </ElButton>
    </div>

    <ElTable :data="files" class="flex-1" border>
      <ElTableColumn min-width="600" label="名称" prop="path">
        <template #default="{ row }">
          <span v-if="!row.is_dir" class="truncate">
            {{ getFileName(row.path) }}
          </span>
          <span v-else class="flex items-center">
            <FolderOpen :size="16" class="mr-2 text-blue-500" />
            <span class="flex-1 truncate">
              {{ getFileName(row.path) }}
            </span>
          </span>
        </template>
      </ElTableColumn>

      <ElTableColumn prop="size" label="大小" width="120">
        <template #default="{ row }">
          <span class="font-mono">
            {{ displaySize(row) }}
          </span>
        </template>
      </ElTableColumn>
    </ElTable>
  </div>
</template>
