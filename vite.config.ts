import process from 'node:process'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

const { TAURI_DEV_HOST } = process.env
console.log(TAURI_DEV_HOST)

export default defineConfig(() => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: TAURI_DEV_HOST || false,
    hmr: TAURI_DEV_HOST
      ? {
          protocol: 'ws',
          host: TAURI_DEV_HOST,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
}))
