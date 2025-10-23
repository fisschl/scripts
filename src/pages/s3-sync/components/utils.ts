/**
 * S3 同步模块工具函数
 */

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

  const lastSlashIndex = Math.max(filePath.lastIndexOf("/"), filePath.lastIndexOf("\\"));

  // 确保最后一个点是在最后一个斜杠之后
  if (lastDotIndex < lastSlashIndex) {
    return "";
  }

  return filePath.slice(lastDotIndex + 1).toLowerCase();
}
