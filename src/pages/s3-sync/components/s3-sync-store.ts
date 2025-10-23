import { Store } from "@tauri-apps/plugin-store";
import { identity, remove } from "lodash-es";
import { defineStore } from "pinia";
import { array, type infer as Infer } from "zod/mini";
import { SyncPlanZod, type SyncPlan } from "./sync-plan";

/** 同步方案数组 Zod 模式定义 */
const SyncPlansArrayZod = array(SyncPlanZod);

/** 同步方案数组类型 */
export type SyncPlanArray = Infer<typeof SyncPlansArrayZod>;

const PLANS_STORAGE_KEY = "s3-sync-plans";
const store = Store.load("sync-plans.json");

/**
 * S3 同步方案管理 Store
 * 负责同步方案的增删改查和持久化存储
 */
export const useS3SyncStore = defineStore("s3Sync", {
  state: () => ({
    /** 同步方案列表状态 */
    syncPlans: identity<SyncPlan[]>([]),
  }),

  actions: {
    /**
     * 初始化时加载保存的同步方案
     */
    async initializePlans() {
      await store.then(async (store) => {
        const data = await store.get(PLANS_STORAGE_KEY);
        // 使用数组schema校验配置数据
        this.syncPlans = SyncPlansArrayZod.parse(data);
      });
    },

    /**
     * 保存方案到存储
     */
    async persistPlans() {
      await store.then(async (store) => {
        await store.set(PLANS_STORAGE_KEY, this.syncPlans);
        await store.save();
      });
    },

    /**
     * 根据ID删除同步方案
     * @param id - 要删除的方案ID
     * @returns 删除是否成功
     */
    async deletePlan(id: string) {
      remove(this.syncPlans, (plan) => plan.id === id);
      return await this.persistPlans();
    },

    /**
     * 根据ID查找同步方案
     * @param id - 要查找的方案ID
     */
    findPlan(id: string) {
      return this.syncPlans.find((plan) => plan.id === id);
    },
  },
});
