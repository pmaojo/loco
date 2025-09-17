import { expect, test } from '@playwright/test';

const graphFixture = {
  routes: [
    { path: '/__loco/graph', methods: ['GET'] },
    { path: '/_health', methods: ['GET'] },
  ],
  dependencies: {
    background_workers: [
      { name: 'worker-1', command: 'bundle exec rake jobs:work', tags: ['queue'] },
    ],
    scheduler_jobs: [
      {
        name: 'cron-1',
        command: 'echo hello',
        schedule: '*/5 * * * * *',
        run_on_start: false,
        shell: true,
        tags: ['base'],
      },
    ],
    tasks: [
      { name: 'seed-db', description: 'Seed the database with fixtures.' },
    ],
  },
  health: { ok: true },
};

test('renders the graph and requests AI guidance', async ({ page }) => {
  let assistantCalled = false;

  await page.route('**/__loco/graph', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(graphFixture),
    });
  });

  await page.route('**/__loco/assistant', async (route) => {
    assistantCalled = true;
    const request = await route.request().postDataJSON();
    expect(request.node.label).toBe('/__loco/graph');
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        summary: 'Route looks healthy.',
        remediationTips: ['Confirm integration tests cover this route.'],
      }),
    });
  });

  await page.goto('/');

  const routeNode = page.locator('[data-node-id="route:0:/__loco/graph"]');
  const routeCircle = routeNode.locator('circle');
  await expect(routeCircle).toBeVisible();
  await page.waitForTimeout(500);
  await routeCircle.click({ force: true });

  const aiButton = page.getByRole('button', { name: 'Request AI guidance' });
  await aiButton.click();

  await expect(page.getByText('Route looks healthy.')).toBeVisible();
  await expect(page.getByText('Confirm integration tests cover this route.')).toBeVisible();
  expect(assistantCalled).toBeTruthy();

  const routesToggle = page.getByLabel('Routes');
  await routesToggle.click();
  await expect(page.locator('[data-node-id^="route:"]')).toHaveCount(0);
});
