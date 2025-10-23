import { object, string, type infer as Infer } from "zod/mini";

/**
 * 同步方案 Zod 模式定义
 */
export const SyncPlanZod = object({
  /** 方案唯一标识符 */
  id: string(),
  /** 存储桶名称 */
  bucket: string(),
  /** S3 实例唯一标识符 */
  s3_instance_id: string(),
  /** 本地目录路径 */
  local_dir: string(),
  /** 远程目录路径 */
  remote_dir: string(),
});

/**
 * 同步方案类型
 */
export type SyncPlan = Infer<typeof SyncPlanZod>;
