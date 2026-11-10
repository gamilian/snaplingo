import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['script/**/*.test.ts', 'reference/web/**/*.{test,spec}.{ts,tsx}'],
  },
});
