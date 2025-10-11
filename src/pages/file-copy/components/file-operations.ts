import { invoke } from '@tauri-apps/api/core'

/**
 * 文件系统条目信息
 *
 * 表示文件或目录的基本元数据信息
 */
export interface FileInfo {
  /** 条目的完整路径 */
  path: string
  /** 是否为目录 */
  is_dir: boolean
  /** 文件大小（字节），目录通常为 0 */
  size: number
  /** 最后修改时间（ISO 8601 格式） */
  last_modified: string
}

/**
 * 列举目录内容
 *
 * 扫描指定目录并返回其中所有文件和子目录的信息。
 * 此操作只列出直接子项，不会深入子目录。
 *
 * @param path - 要扫描的目录路径
 * @returns Promise<FileInfo[]> 返回目录条目信息列表
 *
 * @throws {Error} 当目录不存在、无权限或其他系统错误时抛出异常
 */
export async function listDirectory(path: string): Promise<FileInfo[]> {
  return await invoke<FileInfo[]>('list_directory', { path })
}

/**
 * 递归列举目录中的所有文件
 *
 * 递归扫描指定目录及其所有子目录，返回所有文件的完整路径列表。
 * 过滤掉目录，只返回文件路径。
 *
 * @param dirPath - 要扫描的根目录路径
 * @returns Promise<string[]> 返回所有文件的完整路径列表
 *
 * @throws {Error} 当目录扫描失败时抛出异常
 */
export async function listFilesRecursive(dirPath: string): Promise<string[]> {
  const result: string[] = []

  const files = await listDirectory(dirPath)

  for (const file of files) {
    if (file.is_dir) {
      // 递归处理子目录
      const subFiles = await listFilesRecursive(file.path)
      result.push(...subFiles)
    }
    else {
      // 添加文件路径
      result.push(file.path)
    }
  }

  return result
}

/**
 * 递归列举目录中的所有文件信息
 *
 * 递归扫描指定目录及其所有子目录，返回所有文件的详细信息映射。
 * 返回相对路径到文件信息的映射，自动处理路径分隔符转换。
 *
 * @param dirPath - 要扫描的根目录路径
 * @returns Promise<Map<string, FileInfo>> 返回相对路径到文件信息的映射
 *
 * @throws {Error} 当目录扫描失败时抛出异常
 */
export async function listFilesRecursiveWithInfo(dirPath: string): Promise<Map<string, FileInfo>> {
  const files = new Map<string, FileInfo>()

  const scanDir = async (currentDir: string, relativePath: string = ''): Promise<void> => {
    const entries = await listDirectory(currentDir)

    for (const entry of entries) {
      const entryRelativePath = relativePath
        ? `${relativePath}/${entry.path.split(/[/\\]/).pop()}`
        : entry.path.split(/[/\\]/).pop() || ''

      if (entry.is_dir) {
        // 递归处理子目录
        await scanDir(entry.path, entryRelativePath)
      }
      else {
        // 添加文件信息，使用正斜杠作为路径分隔符
        const normalizedRelativePath = entryRelativePath.replace(/\\/g, '/')
        files.set(normalizedRelativePath, entry)
      }
    }
  }

  await scanDir(dirPath)
  return files
}

/**
 * 检查路径是否为文件
 *
 * @param path - 要检查的路径
 * @returns boolean 如果是文件返回 true，否则返回 false
 */
export function isFile(path: string): boolean {
  return !path.endsWith('/') && !path.endsWith('\\')
}
