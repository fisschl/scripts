import { S3Client } from "bun";

const { S3_ACCESS_KEY_ID, S3_SECRET_ACCESS_KEY, S3_ENDPOINT } = Bun.env;

export const s3 = new S3Client({
  accessKeyId: S3_ACCESS_KEY_ID!,
  secretAccessKey: S3_SECRET_ACCESS_KEY!,
  endpoint: S3_ENDPOINT,
  virtualHostedStyle: true,
});
