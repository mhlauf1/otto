import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { isTauri } from '@tauri-apps/api/core';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './app/App';
import './design/tokens.css';
import './design/global.css';

// Inside the Tauri shell the window is transparent so macOS vibrancy shows through.
document.documentElement.dataset.shell = isTauri() ? 'tauri' : 'browser';

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: false, refetchOnWindowFocus: false } },
});

const container = document.getElementById('root');
if (container) {
  createRoot(container).render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </StrictMode>,
  );
}
