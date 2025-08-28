import { describe, it, expect } from "bun:test";
import { logger } from "./index";

describe("logs 模块", () => {
  it("应该创建一个 logger 实例", () => {
    expect(logger).toBeDefined();
    expect(typeof logger.info).toBe("function");
    expect(typeof logger.error).toBe("function");
    expect(typeof logger.warn).toBe("function");
    expect(typeof logger.debug).toBe("function");
  });

  it("应该能够记录 info 级别日志", () => {
    // 直接调用，如果抛出异常测试会自动失败
    logger.info("测试 info 消息");
  });

  it("应该能够记录 error 级别日志", () => {
    // 直接调用，如果抛出异常测试会自动失败
    logger.error("测试 error 消息");
  });

  it("应该能够记录 warn 级别日志", () => {
    // 直接调用，如果抛出异常测试会自动失败
    logger.warn("测试 warning 消息");
  });

  it("应该能够记录 debug 级别日志", () => {
    // 直接调用，如果抛出异常测试会自动失败
    logger.debug("测试 debug 消息");
  });

  it("应该能够记录对象数据", () => {
    const testData = { userId: 123, action: "test_action" };
    // 直接调用，如果抛出异常测试会自动失败
    logger.info(testData);
  });

  it("应该能够记录带上下文的消息", () => {
    // 直接调用，如果抛出异常测试会自动失败
    logger.info({ requestId: "abc123" }, "处理请求");
  });
});
