<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { cloneDeep } from "lodash-es";
import { Database, Folder } from "lucide-vue-next";
import { v7 } from "uuid";
import { useS3SyncStore } from "./s3-sync-store";
import S3InstanceSelector from "./S3InstanceSelector.vue";
import { SyncPlanZod, type SyncPlan } from "./types";
import type { FormRules } from "element-plus";

const props = defineProps<{
  planId?: string;
}>();

const emit = defineEmits(["success"]);

const visible = defineModel<boolean>("visible");

// 内部表单数据
const formData = ref<Partial<SyncPlan>>({});

// 表单引用
const formRef = useTemplateRef("form-ref");

// 校验规则
const rules = reactive<FormRules>({
  s3_instance_id: [{ required: true, message: "请选择 S3 实例" }],
  bucket: [{ required: true, message: "请输入存储桶名称" }],
  local_dir: [{ required: true, message: "请选择本地目录" }],
  remote_dir: [{ required: true, message: "请输入远程目录路径" }],
});

const store = useS3SyncStore();

const handleOpen = async () => {
  if (!props.planId) {
    formData.value = {};
    await nextTick();
    formRef.value?.clearValidate();
    return;
  }
  const item = store.findPlan(props.planId);
  if (!item) {
    ElMessage.error("未找到该同步方案");
    return;
  }
  formData.value = cloneDeep(item);
};

/**
 * 选择本地目录
 */
async function selectLocalDir() {
  const selected = await open({
    multiple: false,
    directory: true,
  });
  if (!selected) return;
  formData.value.local_dir = selected;
}

/**
 * 保存方案
 */
async function handleSave() {
  await formRef.value?.validate();
  const { id } = formData.value;

  // 规范化 remote_dir
  if (formData.value.remote_dir) {
    formData.value.remote_dir = `${formData.value.remote_dir}/`
      .replace(/^[\\/]+/, "")
      .replaceAll(/[\\/]+/g, "/");
  }

  const handleSuccess = async (item: SyncPlan) => {
    await store.persistPlans();
    visible.value = false;
    emit("success", item);
  };

  if (id) {
    // 更新
    const item = store.findPlan(id);
    if (!item) {
      ElMessage.error("未找到该同步方案");
      return;
    }
    Object.assign(item, formData.value);
    await handleSuccess(item);
    ElMessage.success("更新成功");
  } else {
    // 新增
    const { syncPlans } = store;
    const planData = SyncPlanZod.parse({
      id: v7(),
      ...formData.value,
    });
    syncPlans.push(planData);
    await handleSuccess(planData);
    ElMessage.success("添加成功");
  }
}

const handleCancel = () => {
  visible.value = false;
};
</script>

<template>
  <ElDialog v-model="visible" title="同步方案" width="600px" @open="handleOpen">
    <ElForm
      ref="form-ref"
      :model="formData"
      :rules="rules"
      label-position="top"
      label-suffix="："
      @submit.prevent="handleSave"
    >
      <ElFormItem label="S3 实例" prop="s3_instance_id">
        <S3InstanceSelector v-model="formData.s3_instance_id" />
      </ElFormItem>

      <ElFormItem label="存储桶名称 (Bucket)" prop="bucket">
        <ElInput v-model.trim="formData.bucket" placeholder="请输入 S3 存储桶名称">
          <template #prefix>
            <Database :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="本地目录" prop="local_dir">
        <ElInput
          v-model="formData.local_dir"
          placeholder="点击选择本地目录..."
          @click="selectLocalDir"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <ElFormItem label="远程目录路径" prop="remote_dir">
        <ElInput
          v-model.trim="formData.remote_dir"
          placeholder="例如: website/ 或 backup/2024/ (建议以斜杠结尾)"
        >
          <template #prefix>
            <Folder :size="16" />
          </template>
        </ElInput>
      </ElFormItem>

      <div class="flex justify-end">
        <ElButton native-type="button" @click="handleCancel"> 取消 </ElButton>
        <ElButton type="primary" native-type="submit"> 确定 </ElButton>
      </div>
    </ElForm>
  </ElDialog>
</template>
