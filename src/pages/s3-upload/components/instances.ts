import type { infer as Infer } from 'zod/mini'
import { Store } from '@tauri-apps/plugin-store'
import { array, object, string } from 'zod/mini'

/**
 * S3 实例 Zod 模式定义
 *
 * 定义 S3 实例的验证规则和数据结构
 */
export const S3InstanceZod = object({
  /** S3 服务的终端节点 URL */
  endpoint_url: string(),
  /** AWS 访问密钥 ID */
  access_key_id: string(),
  /** AWS 秘密访问密钥 */
  secret_access_key: string(),
  /** AWS 区域或兼容服务的区域标识 */
  region: string(),
})

/** S3 实例数组 Zod 模式 */
export const S3InstancesZod = array(S3InstanceZod)

/** S3 实例类型推断 */
export type S3Instance = Infer<typeof S3InstanceZod>
/** S3 实例数组类型推断 */
export type S3Instances = Infer<typeof S3InstancesZod>

// S3 配置存储相关常量
const S3_INSTANCES_KEY = 's3-instances'
const S3_CONFIG_FILE = 's3-config.json'

/**
 * 读取 S3 实例列表
 *
 * 从本地存储文件中加载所有已配置的 S3 实例，并进行 Zod 校验。
 * 如果数据格式不正确或不存在，返回空数组。
 *
 * @returns Promise<S3Instances> 返回经过验证的 S3 实例列表
 */
export async function loadS3Instances(): Promise<S3Instances> {
  const store = await Store.load(S3_CONFIG_FILE)
  const data = await store.get(S3_INSTANCES_KEY)
  const result = S3InstancesZod.safeParse(data)
  if (!result.success) {
    return []
  }
  return result.data
}

/**
 * 保存 S3 实例列表
 *
 * 将 S3 实例列表保存到本地存储文件中。
 * 数据会在保存前进行类型校验，确保数据完整性。
 *
 * @param instances - 要保存的 S3 实例列表
 */
export async function saveS3Instances(instances: S3Instances) {
  const store = await Store.load(S3_CONFIG_FILE)
  await store.set(S3_INSTANCES_KEY, instances)
  await store.save()
}
