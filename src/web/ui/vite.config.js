import { defineConfig, transformWithOxc } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

const upstream = path.resolve(import.meta.dirname, 'src/upstream')
const compat = path.resolve(import.meta.dirname, 'src/compat')
const openSse = path.join(upstream, 'open-sse')

export default defineConfig({
  define: { 'process.env.NEXT_PUBLIC_CLOUD_URL': JSON.stringify(process.env.NEXT_PUBLIC_CLOUD_URL || '') },
  plugins: [
    {
      name: 'upstream-jsx-in-js',
      enforce: 'pre',
      async transform(code, id) {
        if (!/[\\/]src[\\/]upstream[\\/].*\.js$/.test(id)) return
        const result = await transformWithOxc(code, id, { lang: 'jsx' })
        return { code: result.code, map: result.map }
      },
    },
    tailwindcss(),
    react(),
  ],
  resolve: {
    alias: {
      '@': upstream,
      'open-sse': openSse,
      'next/navigation': path.join(compat, 'navigation.js'),
      'next/link': path.join(compat, 'link.jsx'),
      'next/image': path.join(compat, 'image.jsx'),
      'next/dynamic': path.join(compat, 'dynamic.jsx'),
    },
  },
  base: '/dashboard/assets/',
  build: {
    outDir: path.resolve(import.meta.dirname, '../assets'),
    emptyOutDir: false,
    rolldownOptions: {
      input: path.resolve(import.meta.dirname, 'index.html'),
      output: {
        entryFileNames: 'dashboard.js',
        chunkFileNames: 'dashboard-[name].js',
        assetFileNames: (assetInfo) => {
          if (assetInfo.name && assetInfo.name.endsWith('.css')) return 'dashboard.css'
          return '[name].[ext]'
        },
      },
    },
  },
})
