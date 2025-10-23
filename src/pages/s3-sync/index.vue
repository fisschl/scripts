<script setup lang="ts">
import { Plus } from "lucide-vue-next";
import { useS3SyncStore } from "./components/s3-sync-store";
import SyncPlanDialog from "./components/SyncPlanDialog.vue";
import SyncProgress from "./components/SyncProgress.vue";
import type { SyncPlan } from "./components/types";

// 使用S3同步方案store
const s3SyncStore = useS3SyncStore();

// 弹窗显示状态
const dialogVisible = ref(false);

const current = ref<SyncPlan>();

s3SyncStore.initializePlans();

/**
 * 显示添加方案弹窗
 */
function showAddPlanDialog() {
  current.value = undefined;
  dialogVisible.value = true;
}

/**
 * 显示编辑方案弹窗
 */
function showEditPlanDialog(item: SyncPlan) {
  current.value = item;
  dialogVisible.value = true;
}

/**
 * 删除方案
 */
async function deletePlan(id: string) {
  await s3SyncStore.deletePlan(id);
  ElMessage.success("方案删除成功");
}

const handleCurrentChange = (item?: SyncPlan) => {
  current.value = item;
};
</script>

<template>
  <div class="flex h-full flex-col p-4">
    <!-- 添加方案按钮 -->
    <div class="mb-4 flex items-center">
      <SyncProgress v-if="current" :plan="current" />

      <p style="flex: 1" />
      <ElButton type="primary" @click="showAddPlanDialog">
        <Plus :size="18" class="mr-2" />
        添加同步方案
      </ElButton>

      <!-- 方案编辑弹窗 -->
      <SyncPlanDialog v-model:visible="dialogVisible" :plan="current?.id" />
    </div>

    <!-- 方案列表表格 -->
    <ElTable
      highlight-current-row
      :data="s3SyncStore.syncPlans"
      border
      class="w-full flex-1"
      show-overflow-tooltip
      @current-change="handleCurrentChange"
    >
      <ElTableColumn prop="remote_dir" label="远程目录" min-width="300" />
      <ElTableColumn prop="local_dir" label="本地目录" min-width="300" />
      <ElTableColumn prop="bucket" label="存储桶" width="180" />
      <ElTableColumn label="编辑" width="150">
        <template #default="{ row }">
          <div class="flex gap-2">
            <ElButton
              type="primary"
              :class="$style.tableButton"
              link
              @click="showEditPlanDialog(row.id)"
            >
              编辑
            </ElButton>
            <ElButton link :class="$style.tableButton" @click="deletePlan(row.id)"> 删除 </ElButton>
          </div>
        </template>
      </ElTableColumn>
    </ElTable>
  </div>
</template>

<style module>
.tableButton + .tableButton {
  margin-left: 0;
}
</style>
