import sharp from "sharp";
import { blake3 } from "@noble/hashes/blake3";
import { base32crockford } from "@scure/base";
import { logger } from "logger";

export { convertToWebM } from "./video";

/**
 * 将图片转换为WebP格式（无损压缩）
 * @param inputPath 输入图片路径
 * @param outputPath 输出WebP图片路径
 */
export const convertToWebP = async (inputPath: string, outputPath: string) => {
  logger.info("开始转换图片为WebP格式...");
  await sharp(inputPath).webp({ lossless: true }).toFile(outputPath);
  logger.info("图片转换完成！");
};

/**
 * 使用Blake3算法计算文件的哈希值
 * @param filePath 文件路径
 * @returns 文件的Blake3哈希值（base32crockford编码字符串）
 */
export const calculateFileHash = async (filePath: string): Promise<string> => {
  const hash = blake3.create();
  const file = Bun.file(filePath);
  const stream = file.stream();
  const reader = stream.getReader();

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      hash.update(value);
    }
  } finally {
    reader.releaseLock();
  }

  // 使用base32crockford编码哈希值
  return base32crockford.encode(hash.digest()).toLowerCase();
};
