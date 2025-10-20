<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { File, Key } from "lucide-vue-next";

/** 选择的文件路径 */
const filePath = ref("");
/** 文件哈希计算结果 */
const hashResult = ref("");

/**
 * 选择文件并计算哈希值
 *
 * 打开文件选择对话框，用户选择文件后自动计算并显示文件的哈希值。
 */
async function selectFile() {
  const selected = await open({
    multiple: false,
    directory: false,
  });
  if (!selected) return;
  filePath.value = selected;
  if (!filePath.value) return;
  const result = await invoke("file_hash", { filePath: filePath.value });
  if (typeof result !== "string") return;
  hashResult.value = result;
}
</script>

<template>
  <div class="p-4">
    <ElForm label-position="top" label-suffix="：">
      <ElFormItem label="选择文件">
        <ElInput
          v-model.trim="filePath"
          placeholder="点击选择文件..."
          :class="$style.input"
          @click="selectFile"
        >
          <template #prefix>
            <File :size="16" />
          </template>
        </ElInput>
      </ElFormItem>
      <ElFormItem v-if="hashResult" label="哈希结果">
        <ElInput :value="hashResult" :class="$style.input">
          <template #prefix>
            <Key :size="16" />
          </template>
        </ElInput>
      </ElFormItem>
    </ElForm>
  </div>
</template>

<style module>
.input {
  max-width: 40rem;
}
</style>
