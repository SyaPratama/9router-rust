import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  base: '/dashboard/assets/',
  build: {
    outDir: path.resolve(import.meta.dirname, '../assets'),
    emptyOutDir: false,
    rollupOptions: {
      input: path.resolve(import.meta.dirname, 'index.html'),
      output: {
        entryFileNames: 'dashboard.js',
        chunkFileNames: 'dashboard-[name].js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name && assetInfo.name.endsWith('.css')) {
            return 'dashboard.css';
          }
          return '[name].[ext]';
        }
      }
    }
  }
})
