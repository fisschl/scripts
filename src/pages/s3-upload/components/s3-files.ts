import { invoke } from '@tauri-apps/api/core'

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

/**
 * S3 对象列表响应
 *
 * 包含分页信息的 S3 对象列表响应结构
 */
export interface ListObjectsResponse {
  /** 对象列表 */
  objects: S3Object[]
  /** 是否还有更多对象 */
  is_truncated: boolean
  /** 用于获取下一页的令牌 */
  next_continuation_token?: string
}

/**
 * 获取所有远程文件列表（分页获取）
 *
 * 递归获取指定 S3 存储桶和前缀下的所有对象，支持分页和进度回调。
 * 自动处理分页逻辑，并返回去除了前缀的相对路径映射。
 *
 * @param endpointUrl - S3 服务的终端节点 URL
 * @param bucket - 存储桶名称
 * @param prefix - 对象键前缀过滤器
 * @param progressCallback - 进度回调函数，接收已处理的对象数量
 * @returns Promise<Map<string, S3Object>> 返回相对路径到 S3 对象的映射
 *
 * @throws {Error} 当 API 调用失败时抛出错误
 */
export async function listAllRemoteFiles(
  endpointUrl: string,
  bucket: string,
  prefix: string,
  progressCallback?: (count: number) => void,
): Promise<Map<string, S3Object>> {
  const remoteFiles = new Map<string, S3Object>()
  let continuationToken: string | undefined
  let totalObjects = 0

  do {
    const response = await invoke<ListObjectsResponse>('list_objects', {
      endpoint_url: endpointUrl,
      bucket,
      prefix,
      continuation_token: continuationToken,
    })

    // 处理当前页的对象
    for (const obj of response.objects) {
      const relativeKey = obj.key.replace(prefix, '')
      if (relativeKey) {
        remoteFiles.set(relativeKey, obj)
      }
    }

    totalObjects += response.objects.length

    // 调用进度回调
    if (progressCallback) {
      progressCallback(totalObjects)
    }

    // 检查是否还有更多数据
    continuationToken = response.next_continuation_token

    // 如果还有更多数据，添加短暂延迟
    if (continuationToken) {
      await new Promise(resolve => setTimeout(resolve, 100))
    }
  } while (continuationToken)

  return remoteFiles
}
