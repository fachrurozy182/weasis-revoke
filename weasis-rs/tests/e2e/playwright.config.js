// End-to-end tests of the Weasis web interface.
//
// The frontend must be built first (`trunk build` in crates/weasis-web); the server is started
// here and serves it on http://127.0.0.1:8080.
const { defineConfig, devices } = require('@playwright/test');

module.exports = defineConfig({
  testDir: './specs',
  outputDir: './test-results',
  timeout: 30_000,
  use: {
    baseURL: 'http://127.0.0.1:8080',
    viewport: { width: 1280, height: 800 },
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'cargo run -q -p weasis-server',
    cwd: '../..',
    url: 'http://127.0.0.1:8080',
    reuseExistingServer: true,
    timeout: 300_000,
  },
});
