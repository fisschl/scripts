import { join } from "path";
import { tmpdir } from "os";
import { rename } from "fs/promises";
import { existsSync } from "fs";
import { v7 } from "uuid";
import sharp from "sharp";
import { $ } from "bun";

export const FFMPEG_PATH = join(tmpdir(), "ffmpeg.exe");

/**
 * 准备FFmpeg可执行文件
 */
export const prepareFFmpegExecutable = async () => {
  if (existsSync(FFMPEG_PATH)) return;
  const ffmpegUrl = "https://bronya.world/static/ffmpeg/bin/ffmpeg.exe";
  const response = await fetch(ffmpegUrl);
  const tmpPath = join(tmpdir(), v7());
  Bun.write(tmpPath, response);
  await rename(tmpPath, FFMPEG_PATH);
};

/**
 * 将图片转换为WebP格式（无损压缩）
 * @param inputPath 输入图片路径
 * @param outputPath 输出WebP图片路径
 */
export const convertToWebP = async (inputPath: string, outputPath: string) => {
  await sharp(inputPath).webp({ lossless: true }).toFile(outputPath);
};

/**
 * 将视频转换为WebM格式（无损转换）
 * @param inputPath 输入视频路径
 * @param outputPath 输出WebM视频路径
 */
export const convertToWebM = async (inputPath: string, outputPath: string) => {
  // 确保FFmpeg可执行文件已准备就绪
  await prepareFFmpegExecutable();

  // 使用FFmpeg进行无损视频转换
  // -c:v libvpx-vp9: 使用VP9视频编码器
  // -crf 0: 恒定质量模式，0为无损
  // -b:v 0: 无码率限制
  // -c:a libopus: 使用Opus音频编码器
  await $`${FFMPEG_PATH} -i ${inputPath} -c:v libvpx-vp9 -crf 0 -b:v 0 -c:a libopus ${outputPath}`;
};
