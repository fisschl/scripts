import { watchImmediate } from "@vueuse/core";
import { cloneDeep, isEqual } from "lodash-es";
import pLimit from "p-limit";

/**
 * 异步加载选项接口
 *
 * @template T - 参数类型
 * @template R - 返回值类型
 */
export interface AsyncLoadOptions<T, R> {
  /** 参数值，可以是响应式引用或getter函数 */
  params: MaybeRefOrGetter<T>;
  /** 获取数据的异步函数 */
  fetcher: (params: T) => Promise<R>;
}

/**
 * 异步数据加载组合式函数
 *
 * 监听参数变化并自动获取数据，支持并发限制和深度比较
 *
 * @template T - 参数类型
 * @template R - 返回值类型
 * @param options - 配置选项
 * @returns 响应式引用，包含加载的结果数据
 */
export function useAsyncLoad<T, R>(
  options: AsyncLoadOptions<T, R>,
): Ref<R | undefined> {
  const limit = pLimit(1);
  const params = computed(() => {
    const { params } = options;
    const value = toValue(params);
    return cloneDeep(value);
  });
  const result = ref<R>();
  watchImmediate(params, (value, oldValue) => {
    if (isEqual(value, oldValue)) return;
    limit(async () => {
      const { fetcher } = options;
      result.value = await fetcher(value);
    });
  });
  return result;
}
