import { defineConfig, devices } from '@playwright/test';

// M0 smoke test runs against the Vite dev server in WebKit (the same engine as Tauri on macOS).
export default defineConfig({
  testDir: 'tests/e2e',
  use: { baseURL: 'http://localhost:1420' },
  projects: [{ name: 'webkit', use: { ...devices['Desktop Safari'] } }],
  webServer: { command: 'pnpm dev:web', port: 1420, reuseExistingServer: !process.env.CI },
});
