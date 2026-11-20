import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { RouterProvider } from '@tanstack/react-router'
import { ErrorBoundary } from './app/error-boundary'
import { router } from './app/router'
import './index.css'

const host = document.getElementById('root')
if (!host) throw new Error('#root missing')

createRoot(host).render(
  <StrictMode>
    <ErrorBoundary>
      <RouterProvider router={router} />
    </ErrorBoundary>
  </StrictMode>,
)
