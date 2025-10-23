/**
 * S3 同步模块统一类型定义
 */

/**
 * S3 对象元数据
 *
 * 表示 S3 存储桶中对象的基本信息
 */
import { object, string, type infer as Infer } from "zod/mini";

export interface S3Object {
  /** 对象在 S3 中的唯一键 */
  key: string;
  /** 对象大小（字节） */
  size?: number;
  /** 最后修改时间的 ISO 8601 格式字符串 */
  last_modified?: string;
}

/**
 * 文件系统条目信息
 *
 * 表示文件或目录的基本元数据信息
 */
export interface FileInfo {
  /** 条目的完整路径 */
  path: string;
  /** 是否为目录 */
  is_dir: boolean;
  /** 文件大小（字节），目录通常为 0 */
  size: number;
  /** 最后修改时间（ISO 8601 格式） */
  last_modified: string;
}

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
