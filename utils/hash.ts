import { blake3 } from "@noble/hashes/blake3";
import { base32crockford } from "@scure/base";

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