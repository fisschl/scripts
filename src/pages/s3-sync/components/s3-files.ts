/**
 * S3 对象元数据
 *
 * 表示 S3 存储桶中对象的基本信息
 */
export interface S3Object {
  /** 对象在 S3 中的唯一键 */
  key: string
  /** 对象大小（字节） */
  size?: number
  /** 最后修改时间的 ISO 8601 格式字符串 */
  last_modified?: string
}
