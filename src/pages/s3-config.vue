<script setup lang="ts">
import type { FormRules } from 'element-plus'
import type { infer as Infer } from 'zod/mini'
import { Store } from '@tauri-apps/plugin-store'
import { cloneDeep, remove } from 'lodash-es'
import { Globe, Key, MapPin, Plus } from 'lucide-vue-next'
import { array, object, string } from 'zod/mini'

// S3 实例 Zod 模式定义
const S3InstanceZod = object({
  endpoint_url: string(),
  access_key_id: string(),
  secret_access_key: string(),
  region: string(),
})

const S3InstancesZod = array(S3InstanceZod)

interface S3Instance extends Infer<typeof S3InstanceZod> {}

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

const STORE_KEY = 's3-instances'
const store = Store.load('s3-config.json')

// 加载 S3 实例列表
async function loadInstances() {
  const data = await store.then(store => store.get(STORE_KEY))
  const result = S3InstancesZod.safeParse(data)
  if (!result.success) {
    instances.value = []
    return
  }
  instances.value = result.data
}
loadInstances()

// 保存 S3 实例列表
async function saveInstances() {
  await store.then(async (store) => {
    await store.set(STORE_KEY, instances.value)
    await store.save()
  })
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
  const data = instances.value.find(item => item.endpoint_url === form.value.endpoint_url)
  if (data) {
    Object.assign(data, form.value)
    return
  }
  const result = S3InstanceZod.parse(form.value)
  instances.value.push(result)

  await saveInstances()
  ElMessage.success('保存成功')
  showFormDialog.value = false
}

// 删除实例
async function deleteInstance(instance: S3Instance) {
  await ElMessageBox.confirm(
    `确定删除 "${instance.region}" 吗？`,
    '删除确认',
  )
  remove(instances.value, item => item.endpoint_url === instance.endpoint_url)
  await saveInstances()
  ElMessage.success('删除成功')
}
</script>

<template>
  <div class="p-4 flex flex-col h-full min-h-0">
    <div class="mb-4 flex items-center justify-end">
      <ElButton type="primary" @click="openAddForm">
        <Plus :size="18" class="mr-2" />
        添加实例
      </ElButton>
    </div>

    <!-- 实例列表 -->
    <ElEmpty v-if="instances.length === 0" description="暂无 S3 实例配置" class="py-12" />

    <ElTable v-else :data="instances" border style="width: 100%" class="flex-1">
      <ElTableColumn type="index" label="序号" width="80" />
      <ElTableColumn prop="endpoint_url" label="Endpoint URL" min-width="300" />
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
      width="600px"
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
            style="width: 22rem;"
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
            style="width: 16rem;"
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
  </div>
</template>
