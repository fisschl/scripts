import sharp from "sharp";
import { logger } from "./logger";
import { calculateFileHash } from "./hash";

export { convertToWebM } from "./video";
export { calculateFileHash } from "./hash";

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