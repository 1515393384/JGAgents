import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// 开发期只代理 API 前缀，绝不影响前端路由。
// 约定：页面路由 /admin 归前端；管理 API 一律 /api/admin/*。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  // 启动时就预构建着色器依赖。
  // 不写这一条，Vite 会在会话中途才发现它 → 重新预构建 → 旧模块 URL 作废
  // → 浏览器拿到 MIME 为空的响应 → 白屏。
  optimizeDeps: {
    include: ['@paper-design/shaders-react'],
  },
  server: {
    port: 5173,
    // 启动即预热入口，避免首次访问撞上"正在转换"的空窗
    warmup: {
      clientFiles: ['./src/main.tsx'],
    },
    proxy: {
      '/rpc': { target: 'http://127.0.0.1:3000', ws: true },
      '/api': { target: 'http://127.0.0.1:3000' },
      '/healthz': { target: 'http://127.0.0.1:3000' },
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    sourcemap: false,
  },
})
