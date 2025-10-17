<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { S3Instance } from './instances'
import { cloneDeep, remove } from 'lodash-es'
import { Globe, Key, MapPin, Plus } from 'lucide-vue-next'
import { v7 as uuidv7 } from 'uuid'
import { loadS3Instances, S3InstanceZod, saveS3Instances } from './instances'

interface Emits {
  (e: 'close'): void
}

const emit = defineEmits<Emits>()

// 使用 defineModel 创建双向绑定的 model
const visible = defineModel<boolean>({ default: false })

// 校验规则
const rules = reactive<FormRules>({
  endpoint_url: [
    { required: true, message: '请输入 Endpoint URL' },
    { type: 'url', message: '请输入有效的 URL 地址' },
  ],
  access_key_id: [
    { required: true, message: '请输入 Access Key ID' },
    { min: 1, message: 'Access Key ID 不能为空' },
  ],
  secret_access_key: [
    { required: true, message: '请输入 Secret Access Key' },
    { min: 1, message: 'Secret Access Key 不能为空' },
  ],
  region: [
    { required: true, message: '请输入 Region' },
    { min: 1, message: 'Region 不能为空' },
  ],
})

const instances = ref<S3Instance[]>([])
const showFormDialog = ref(false)

// 加载 S3 实例列表
async function loadInstances() {
  instances.value = await loadS3Instances()
}

// 保存 S3 实例列表
async function saveInstances() {
  await saveS3Instances(instances.value)
}

// 打开新增表单
function openAddForm() {
  resetForm()
  showFormDialog.value = true
}

// 表单数据
const form = ref<Partial<S3Instance>>({})

// 表单引用
const formRef = useTemplateRef('form-ref')

// 打开编辑表单
function openEditForm(instance: S3Instance) {
  form.value = cloneDeep(instance)
  showFormDialog.value = true
}

// 重置表单
async function resetForm() {
  form.value = {}
  await nextTick()
  formRef.value?.clearValidate()
}

// 提交表单
async function submitForm() {
  await formRef.value?.validate()

  const { s3_instance_id } = form.value

  // 判断是编辑还是新增
  if (s3_instance_id) {
    // 编辑模式：根据 s3_instance_id 查找并更新现有实例
    const item = instances.value.find(item => item.s3_instance_id === s3_instance_id)
    if (item)
      Object.assign(item, form.value)
    else
      ElMessage.error('实例不存在')
  }
  else {
    // 新增模式：创建新的 S3 实例
    const result = S3InstanceZod.parse({
      ...form.value,
      s3_instance_id: uuidv7(),
    })
    instances.value.push(result)
  }

  await saveInstances()
  ElMessage.success('保存成功')
  showFormDialog.value = false
}

// 删除实例
async function deleteInstance(instance: S3Instance) {
  await ElMessageBox.confirm(
    `确定删除 "${instance.endpoint_url}" 吗？`,
    '删除确认',
  )
  remove(instances.value, item => item.s3_instance_id === instance.s3_instance_id)
  await saveInstances()
  ElMessage.success('删除成功')
}
</script>

<template>
  <ElDrawer
    v-model="visible"
    title="S3 实例配置"
    size="800px"
    direction="rtl"
    append-to-body
    :header-class="$style.drawerHeader"
    :body-class="$style.drawerBody"
    @close="emit('close')"
    @open="loadInstances"
  >
    <!-- 添加实例按钮 -->
    <div class="mb-4 flex justify-end">
      <ElButton type="primary" @click="openAddForm">
        <Plus :size="18" class="mr-2" />
        添加实例
      </ElButton>
    </div>

    <!-- 实例列表 -->
    <ElEmpty v-if="instances.length === 0" description="暂无 S3 实例配置" class="py-12" />

    <ElTable v-else :data="instances" border style="width: 100%" class="flex-1">
      <ElTableColumn type="index" label="序号" width="60" />
      <ElTableColumn prop="endpoint_url" label="Endpoint URL" min-width="200" />
      <ElTableColumn label="操作" width="150">
        <template #default="{ row }">
          <ElButton
            type="primary"
            link
            @click="openEditForm(row)"
          >
            编辑
          </ElButton>
          <ElButton
            type="info"
            link
            @click="deleteInstance(row)"
          >
            删除
          </ElButton>
        </template>
      </ElTableColumn>
    </ElTable>

    <!-- 添加/编辑对话框 -->
    <ElDialog
      v-model="showFormDialog"
      title="编辑 S3 实例"
      width="500px"
      append-to-body
    >
      <ElForm
        ref="form-ref"
        :model="form"
        :rules="rules"
        label-position="top"
        label-suffix="："
        @submit.prevent="submitForm"
      >
        <ElFormItem label="Endpoint URL" prop="endpoint_url">
          <ElInput
            v-model.trim="form.endpoint_url"
            placeholder="例如: https://tos-s3-cn-shanghai.volces.com"
          >
            <template #prefix>
              <Globe :size="16" />
            </template>
          </ElInput>
        </ElFormItem>

        <ElFormItem label="Access Key ID" prop="access_key_id">
          <ElInput
            v-model.trim="form.access_key_id"
            placeholder="请输入 AWS Access Key ID"
          >
            <template #prefix>
              <Key :size="16" />
            </template>
          </ElInput>
        </ElFormItem>

        <ElFormItem label="Secret Access Key" prop="secret_access_key">
          <ElInput
            v-model.trim="form.secret_access_key"
            placeholder="请输入 AWS Secret Access Key"
            type="password"
            show-password
          >
            <template #prefix>
              <Key :size="16" />
            </template>
          </ElInput>
        </ElFormItem>

        <ElFormItem label="Region" prop="region">
          <ElInput
            v-model.trim="form.region"
            placeholder="例如: tos-s3-cn-shanghai"
          >
            <template #prefix>
              <MapPin :size="16" />
            </template>
          </ElInput>
        </ElFormItem>

        <div class="flex justify-end">
          <ElButton native-type="button" @click="showFormDialog = false">
            取消
          </ElButton>
          <ElButton type="primary" native-type="submit">
            保存
          </ElButton>
        </div>
      </ElForm>
    </ElDialog>
  </ElDrawer>
</template>

<style module>
.drawerHeader {
  margin-bottom: 0;
}

.drawerBody {
  display: flex;
  flex-direction: column;
}
</style>
