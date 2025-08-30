import { describe, it, expect, beforeEach, afterEach } from "bun:test";
import { calculateFileHash } from "./ffmpeg";
import { writeFileSync, rmSync } from "fs";
import { resolve } from "path";

describe("calculateFileHash", () => {
  const testFilePath = resolve("./test-file.txt");
  const testContent = "This is a test file for hashing";

  beforeEach(() => {
    // 创建测试文件
    writeFileSync(testFilePath, testContent);
  });

  afterEach(() => {
    // 清理测试文件
    rmSync(testFilePath);
  });

  it("应该计算文件的Blake3哈希值", async () => {
    const hash = await calculateFileHash(testFilePath);

    // 验证返回值是一个字符串
    expect(typeof hash).toBe("string");

    // 验证返回值不为空
    expect(hash.length).toBeGreaterThan(0);

    // 验证返回值是小写的
    expect(hash).toBe(hash.toLowerCase());
  });

  it("相同内容的文件应该产生相同的哈希值", async () => {
    // 创建另一个具有相同内容的文件
    const testFile2Path = resolve("./test-file2.txt");
    writeFileSync(testFile2Path, testContent);

    const hash1 = await calculateFileHash(testFilePath);
    const hash2 = await calculateFileHash(testFile2Path);

    // 清理第二个测试文件
    rmSync(testFile2Path);

    // 验证两个相同内容的文件产生相同的哈希值
    expect(hash1).toBe(hash2);
  });

  it("不同内容的文件应该产生不同的哈希值", async () => {
    // 创建另一个具有不同内容的文件
    const testFile3Path = resolve("./test-file3.txt");
    const differentContent = "This is a different test file for hashing";
    writeFileSync(testFile3Path, differentContent);

    const hash1 = await calculateFileHash(testFilePath);
    const hash2 = await calculateFileHash(testFile3Path);

    // 清理第三个测试文件
    rmSync(testFile3Path);

    // 验证两个不同内容的文件产生不同的哈希值
    expect(hash1).not.toBe(hash2);
  });
});
