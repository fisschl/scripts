/**
 * 文件项信息
 */
export interface FileItem {
  /** 文件路径 */
  path: string
  /** 文件状态：等待处理或正在处理 */
  status: 'pending' | 'processing'
}

/**
 * 文件详细信息
 */
export interface FileInfo {
  /** 文件路径 */
  path: string
  /** 是否为目录 */
  is_dir: boolean
  /** 文件大小（字节） */
  size: number
}
