export {};

if (process?.env?.VITEST) {
  const { expect } = await import('vitest');
  const { default: matchers } = await import('@testing-library/jest-dom/matchers');
  expect.extend(matchers);
}
