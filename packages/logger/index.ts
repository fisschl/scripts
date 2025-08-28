import pino from "pino";
import { join } from "path";
import { format } from "date-fns";

const logFile = join(
  import.meta.dir,
  "logs",
  `${format(new Date(), "yyyyMMddHHmmss")}.log`
);

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
          mkdir: true, // pino/file 会自动创建目录和文件
        },
      },
    ],
  },
});
