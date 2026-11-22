import { defineConfig, loadEnv } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, '.', '')
  const port = Number(env.PORT || 3000)
  const apiTarget = env.API_TARGET?.trim()

  return {
    plugins: [react(), tailwindcss()],
    optimizeDeps: {
      include: ['@paper-design/shaders-react'],
    },
    server: {
      host: '0.0.0.0',
      port,
      strictPort: true,
      allowedHosts: true,
      hmr: false,
      warmup: {
        clientFiles: ['./src/main.tsx'],
      },
      ...(apiTarget
        ? {
            proxy: {
              '/rpc': { target: apiTarget, ws: true },
              '/api': { target: apiTarget },
              '/healthz': { target: apiTarget },
            },
          }
        : {}),
    },
    build: {
      outDir: 'dist',
      emptyOutDir: true,
      sourcemap: false,
    },
  }
})
