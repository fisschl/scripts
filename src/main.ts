import { createPinia } from "pinia";
import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import { routes } from "vue-router/auto-routes";
import App from "./App.vue";
import "@fontsource-variable/fira-code";
import "element-plus/theme-chalk/dark/css-vars.css";
import "./style.css";
import "./assets/MiSans/mi-sans.css";

/**
 * Vue 应用程序入口点
 *
 * 创建并挂载 Vue 应用实例，配置路由系统，并导入全局样式。
 */

// 从环境变量获取基础 URL
const { BASE_URL } = import.meta.env;

// 创建 Vue Router 实例
const router = createRouter({
  history: createWebHistory(BASE_URL),
  routes,
});

// 创建 Pinia 实例
const pinia = createPinia();

// 创建并挂载 Vue 应用
createApp(App).use(router).use(pinia).mount("#app");
