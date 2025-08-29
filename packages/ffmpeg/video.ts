import { $ } from "bun";
import { existsSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { v7 } from "uuid";
import { logger } from "logger";

const FFMPEG_PATH = join(tmpdir(), "ffmpeg.exe");
const FFPROBE_PATH = join(tmpdir(), "ffprobe.exe");

/**
 * 准备FFmpeg可执行文件
 */
const prepareFFmpegExecutable = async () => {
  if (!existsSync(FFMPEG_PATH)) {
    logger.info("开始下载FFmpeg可执行文件...");
    // 使用PowerShell的Invoke-WebRequest直接下载文件
    const ffmpegUrl = "https://bronya.world/static/ffmpeg/bin/ffmpeg.exe";
    await $`powershell -Command "Invoke-WebRequest -Uri ${ffmpegUrl} -OutFile ${FFMPEG_PATH}"`;
    logger.info("FFmpeg可执行文件下载完成");
  }

  if (!existsSync(FFPROBE_PATH)) {
    logger.info("开始下载FFprobe可执行文件...");
    // 使用PowerShell的Invoke-WebRequest直接下载文件
    const ffprobeUrl = "https://bronya.world/static/ffmpeg/bin/ffprobe.exe";
    await $`powershell -Command "Invoke-WebRequest -Uri ${ffprobeUrl} -OutFile ${FFPROBE_PATH}"`;
    logger.info("FFprobe可执行文件下载完成");
  }
};

/**
 * 获取视频的时长信息
 * @param inputPath 输入视频路径
 * @returns 视频时长（秒）或 undefined（如果无法获取）
 */
const getVideoDuration = async (
  inputPath: string
): Promise<number | undefined> => {
  // 确保FFmpeg可执行文件已准备就绪
  await prepareFFmpegExecutable();

  // 使用ffprobe获取视频时长信息
  const result =
    await $`${FFPROBE_PATH} -v error -show_entries format=duration -of default=nw=1 ${inputPath}`.text();
  const durationMatch = result.match(/duration=(\d+\.?\d*)/);

  if (durationMatch && durationMatch[1]) {
    return parseFloat(durationMatch[1]);
  }

  // 如果无法获取时长，返回 undefined
  return undefined;
};

/**
 * 根据视频大小和时长估算比特率，如果无法估算则返回默认值
 * @param inputPath 输入视频路径
 * @returns 估算的比特率（Mbps）向上取整，如果无法估算则返回默认值8
 */
const estimateBitrate = async (inputPath: string): Promise<number> => {
  // 首先尝试获取原视频比特率
  // 确保FFmpeg可执行文件已准备就绪
  await prepareFFmpegExecutable();

  // 使用ffprobe获取视频比特率信息（单位为bps）
  const result =
    await $`${FFPROBE_PATH} -v error -select_streams v:0 -show_entries stream=bit_rate -of default=nw=1 ${inputPath}`.text();
  const bitrateMatch = result.match(/bit_rate=(\d+)/);

  if (bitrateMatch && bitrateMatch[1]) {
    const bitrateBps = parseInt(bitrateMatch[1], 10);
    // 转换为Mbps并向上取整
    const result = Math.ceil(bitrateBps / 1000000);
    logger.info(`确切计算比特率 ${result}M`);
    return result;
  }

  // 如果无法获取原视频比特率，则从视频大小和时长估算
  // 获取文件大小
  const file = Bun.file(inputPath);
  const fileSize = file.size;

  // 获取视频时长
  const duration = await getVideoDuration(inputPath);

  if (fileSize && duration) {
    // 比特率（bps）= 文件大小（bits）/ 时长（秒）
    // 文件大小从字节转换为bits：bytes * 8
    const bitrateBps = (fileSize * 8) / duration;
    // 转换为Mbps并向上取整
    return Math.ceil(bitrateBps / 1000000);
  }

  // 如果无法估算比特率，返回默认值8
  return 8;
};

/**
 * 将视频转换为WebM格式（AV1编码，2-pass + VBR）
 * @param inputPath 输入视频路径
 * @param outputPath 输出WebM视频路径
 */
export const convertToWebM = async (inputPath: string, outputPath: string) => {
  // 确保FFmpeg可执行文件已准备就绪
  await prepareFFmpegExecutable();

  // 获取估算的比特率（estimateBitrate 一定会返回数值）
  const estimatedBitrate = await estimateBitrate(inputPath);
  const bitrateToUse = `${estimatedBitrate}M`;
  logger.info(`使用估算比特率: ${estimatedBitrate}M`);

  // 生成唯一的日志文件路径，使用系统临时目录和uuid v7
  const logFilePath = join(tmpdir(), v7());
  logger.info(`使用临时日志文件路径: ${logFilePath}`);

  logger.info("开始第一遍AV1编码（分析）...");
  // 第一遍：分析视频，生成统计信息
  // 使用AV1编码器，VBR模式，指定日志文件路径
  await $`${FFMPEG_PATH} -i ${inputPath} -c:v libaom-av1 -b:v ${bitrateToUse} -pass 1 -passlogfile ${logFilePath} -f webm NUL -y -v quiet -stats`;

  logger.info("开始第二遍AV1编码（生成输出）...");
  // 第二遍：使用统计信息生成最终输出
  await $`${FFMPEG_PATH} -i ${inputPath} -c:v libaom-av1 -b:v ${bitrateToUse} -pass 2 -passlogfile ${logFilePath} -c:a libopus ${outputPath} -y -v quiet -stats`;

  logger.info("AV1 2-pass编码完成！");
};
