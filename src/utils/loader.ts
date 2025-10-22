import { watchImmediate } from "@vueuse/core";
import { cloneDeep, isEqual } from "lodash-es";
import { hash } from "ohash";
import type { LRUCache } from "lru-cache";
import type { LimitFunction } from "p-limit";

/**
 * 异步数据加载器配置选项接口
 * 用于定义useLoader组合式函数的配置参数
 *
 * @template T - 参数类型，可以是任何类型，用于传递给fetcher函数
 * @template R - 返回值类型，必须是对象类型（包括数组、函数等非原始类型）
 */
export interface LoaderOptions<T, R extends object> {
  /**
   * 请求参数，可以是响应式引用或getter函数
   * 当参数变化时，会自动触发数据重新加载
   */
  params: MaybeRefOrGetter<T>;

  /**
   * 数据获取函数，负责从数据源获取数据
   * @param params - 当前的参数值
   * @returns Promise，解析为获取的数据
   */
  fetcher: (params: T) => Promise<R>;

  /**
   * 可选的LRU缓存实例，用于缓存已获取的数据
   * 使用ohash生成缓存键，避免重复请求相同数据
   */
  cache?: LRUCache<string, R>;

  /**
   * 可选的并发限制函数，用于控制并发请求数量
   * 通常使用p-limit库创建
   */
  limit?: LimitFunction;
}

/**
 * 异步数据加载组合式函数
 * Vue 3组合式API，用于响应式数据加载、自动刷新和缓存管理
 *
 * 特性：
 * - 自动监听参数变化并触发数据刷新
 * - 支持数据缓存，避免重复请求
 * - 提供并发请求限制
 * - 深度比较参数，避免不必要的刷新
 * - 管理加载状态
 *
 * @template T - 参数类型，可以是任何类型
 * @template R - 返回值类型，必须是对象类型（object约束，包括数组、对象字面量等）
 * @param options - 加载器配置选项
 * @returns 响应式对象，包含数据、加载状态和刷新方法
 *
 * @example
 * ```typescript
 * const { data, isLoading, refresh } = useLoader({
 *   params: computed(() => ({ id: props.userId })),
 *   fetcher: async (params) => fetchUserDetails(params.id),
 *   cache: userCache,
 *   limit: pLimit(1)
 * });
 * ```
 */
export function useLoader<T, R extends object>(options: LoaderOptions<T, R>) {
  /**
   * 计算属性，用于获取当前的参数值
   * 使用cloneDeep创建深拷贝，避免外部修改影响内部状态
   */
  const params = computed(() => {
    const { params } = options;
    const value = toValue(params); // 解析响应式引用或getter函数
    return cloneDeep(value); // 创建深拷贝，防止外部对象修改
  });

  /**
   * 响应式引用，存储加载的数据
   * 类型为R，初始值为undefined
   */
  const data = ref<R>();

  /**
   * 响应式引用，跟踪数据加载状态
   * true表示正在加载，false表示加载完成或未开始
   */
  const isLoading = ref(false);

  /**
   * 计算属性，生成缓存键
   * 仅当提供了缓存实例时才生成
   * 使用ohash对参数进行哈希处理
   */
  const cacheKey = computed(() => {
    const { cache } = options;
    return cache && hash(params.value); // 只有在提供缓存时才生成缓存键
  });

  /**
   * 内部数据获取函数
   * 负责调用fetcher获取数据，并更新状态
   */
  const fetchData = async () => {
    try {
      isLoading.value = true;
      const { fetcher, cache } = options;
      const result = await fetcher(params.value); // 调用用户提供的数据获取函数

      // 如果配置了缓存且有有效的缓存键，则缓存结果
      if (cache && cacheKey.value) cache.set(cacheKey.value, result);

      data.value = result; // 更新数据状态
    } finally {
      // 确保无论成功失败，都会设置loading为false
      isLoading.value = false;
    }
  };

  /**
   * 监听参数变化，自动触发数据刷新
   * 立即执行一次监听回调，确保初始加载
   */
  watchImmediate(params, (value, oldValue) => {
    // 如果参数值没有变化（深度比较），则不重新加载
    if (isEqual(value, oldValue)) return;

    const { cache } = options;

    // 检查缓存中是否已有结果
    if (cache && cacheKey.value) {
      const result = cache.get(cacheKey.value);
      if (result) {
        // 如果缓存命中，直接使用缓存结果
        data.value = result;
        return;
      }
    }

    // 根据是否配置了并发限制，选择合适的方式调用fetchData
    const { limit } = options;
    if (limit) limit(fetchData);
    else fetchData();
  });

  /**
   * 手动刷新数据的方法
   * 忽略缓存，强制重新获取数据
   * @returns Promise，解析为undefined
   */
  const refresh = async () => {
    const { limit } = options;
    if (limit) await limit(fetchData);
    else await fetchData();
  };

  /**
   * 返回响应式对象，包含：
   * - data: 加载的数据
   * - refresh: 手动刷新方法
   * - isLoading: 加载状态
   */
  return reactive({
    data,
    refresh,
    isLoading,
  });
}
