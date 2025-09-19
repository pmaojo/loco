# Agent Instructions for `graph-gui`

## Vitest setup
- Keep `tests/setup.ts` registering Testing Library matchers with a static namespace import: `import * as matchers from '@testing-library/jest-dom/matchers';` followed by `expect.extend(matchers);`. Dynamic imports cause `expect` to be undefined in this environment.
- When adjusting the Vitest bootstrap, continue to import `expect` directly from `vitest` rather than relying on runtime globals. This prevents `pnpm vitest run tests/unit/useCommandConsole.test.ts` from failing due to `expect.extend` receiving `undefined`.
