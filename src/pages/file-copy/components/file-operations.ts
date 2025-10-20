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
 * 获取文件扩展名
 *
 * 从文件路径中提取扩展名，返回小写形式
 * 注意：返回的是**不带点**的扩展名（例如：返回 "jpg" 而不是 ".jpg"）
 * 如果文件没有扩展名则返回空字符串
 *
 * @param filePath - 文件路径
 * @returns string 返回文件扩展名（小写，不带点），如果没有扩展名则返回空字符串
 */
export function getFileExtension(filePath: string): string {
  const lastDotIndex = filePath.lastIndexOf(".");
  if (lastDotIndex === -1 || lastDotIndex === filePath.length - 1) {
    return "";
  }

  const lastSlashIndex = Math.max(
    filePath.lastIndexOf("/"),
    filePath.lastIndexOf("\\"),
  );

  // 确保最后一个点是在最后一个斜杠之后
  if (lastDotIndex < lastSlashIndex) {
    return "";
  }

  return filePath.slice(lastDotIndex + 1).toLowerCase();
}
