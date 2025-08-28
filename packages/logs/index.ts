import pino from "pino";
import { mkdir } from "fs/promises";
import { join } from "path";

// 确保 tmp 目录存在
const logDir = join(process.cwd(), "tmp");
const logFile = join(logDir, "app.log");

// 初始化日志目录
const initLogDir = async () => {
  try {
    await mkdir(logDir, { recursive: true });
  } catch (error) {
    console.error("Failed to create log directory:", error);
  }
};

// 在模块加载时初始化日志目录
initLogDir();

// 创建 logger 实例，同时输出到控制台和文件
export const logger = pino({
  transport: {
    targets: [
      {
        target: "pino-pretty",
        options: {
          colorize: true,
        },
      },
      {
        target: "pino/file",
        options: {
          destination: logFile,
          mkdir: true,
        },
      },
    ],
  },
});
