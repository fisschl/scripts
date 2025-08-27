import { S3Client, GetObjectCommand } from "@aws-sdk/client-s3";
import { mkdir } from "fs/promises";
import { dirname } from "path";

export const createS3Client = (configs: {
  accessKeyId: string;
  secretAccessKey: string;
}) => {
  return new S3Client({
    region: "tos-s3-cn-shanghai",
    credentials: {
      accessKeyId: configs.accessKeyId,
      secretAccessKey: configs.secretAccessKey,
    },
    endpoint: "https://tos-s3-cn-shanghai.volces.com",
    forcePathStyle: false,
  });
};

/**
 * 从 S3 下载文件到指定目录
 * @param client S3 客户端实例
 * @param params 下载参数
 */
export const downloadFileFromS3 = async (
  client: S3Client,
  params: {
    bucket: string;
    key: string;
    localFilePath: string;
  }
) => {
  const { bucket, key, localFilePath } = params;
  // 确保目标目录存在
  const directory = dirname(localFilePath);
  await mkdir(directory, { recursive: true });

  // 创建 GetObjectCommand
  const command = new GetObjectCommand({
    Bucket: bucket,
    Key: key,
  });
  const response = await client.send(command);
  const webStream = response.Body?.transformToWebStream();
  if(!webStream) throw new Error("无法获取文件内容");
  await Bun.write(localFilePath, new Response(webStream));
};
