import { calculateFileHash } from "ffmpeg";
import { existsSync } from "fs";
import { access, mkdir, readdir, rename, stat } from "fs/promises";
import { logger } from "logger";
import { extname, join, resolve } from "path";

const inputDir = resolve("./input");
logger.info(`输入目录: ${inputDir}`);
if (!existsSync(inputDir)) throw new Error(`输入目录不存在`);

const outputDir = resolve("./output");
if (!existsSync(outputDir)) await mkdir(outputDir);

/**
 * 异步递归遍历目录内的所有文件
 * @param directoryPath 要遍历的目录路径
 * @returns 文件绝对路径数组的Promise
 */
export const traverseDirectory = async (
  directoryPath: string
): Promise<string[]> => {
  const files: string[] = [];

  // 检查目录是否存在
  await access(directoryPath);

  // 读取目录内容
  const entries = await readdir(directoryPath);

  for (const entry of entries) {
    const fullPath = join(directoryPath, entry);

    // 获取文件状态
    const stats = await stat(fullPath);

    if (stats.isFile()) {
      // 如果是文件，添加到结果数组
      files.push(fullPath);
    } else if (stats.isDirectory()) {
      // 如果是目录，递归遍历并合并结果
      const subFiles = await traverseDirectory(fullPath);
      files.push(...subFiles);
    }
  }

  return files;
};

const inputFiles = await traverseDirectory(inputDir);

const ImageExtensions = [
  ".jpg",
  ".jpeg",
  ".png",
  ".webp",
  ".svg",
  ".avif",
  ".ico",
];

const inputImages = inputFiles.filter((file) => {
  const extension = extname(file).toLowerCase();
  return ImageExtensions.includes(extension);
});

const VideoExtensions = [".mp4", ".webm", ".ogg", ".ogv"];

const inputVideos = inputFiles.filter((file) => {
  const extension = extname(file).toLowerCase();
  return VideoExtensions.includes(extension);
});

for (const file of [...inputImages, ...inputVideos]) {
  const hashValue = await calculateFileHash(file);
  const outputPath = join(outputDir, `${hashValue}${extname(file)}`);
  await rename(file, outputPath);
}
