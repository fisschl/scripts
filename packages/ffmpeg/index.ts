import { join } from "path";
import { tmpdir } from "os";
import { rename } from "fs/promises";
import { existsSync } from "fs";
import { v7 } from "uuid";

/**
 * 准备FFmpeg可执行文件
 */
export const prepareFFmpegExecutable = async () => {
  const ffmpegPath = join(tmpdir(), "ffmpeg.exe");
  if (existsSync(ffmpegPath)) return;
  const ffmpegUrl = "https://bronya.world/static/ffmpeg/bin/ffmpeg.exe";
  const response = await fetch(ffmpegUrl);
  const tmpPath = join(tmpdir(), `${v7()}.exe`);
  Bun.write(tmpPath, response);
  await rename(tmpPath, ffmpegPath);
};
