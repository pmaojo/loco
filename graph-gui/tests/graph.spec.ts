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
  let generatorRunCalled = false;
  let taskRunCalled = false;

  await page.route('**/__loco/graph', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify(graphFixture),
    });
  });

  await page.route('**/__loco/cli/generators', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { command: 'model', summary: 'Generate a new model' },
        { command: 'migration', summary: 'Generate a new migration' },
      ]),
    });
  });

  await page.route('**/__loco/cli/tasks', async (route) => {
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify([
        { command: 'seed', summary: 'Seed data' },
        { command: 'refresh', summary: 'Refresh materialized views' },
      ]),
    });
  });

  await page.route('**/__loco/cli/generators/run', async (route) => {
    generatorRunCalled = true;
    const payload = await route.request().postDataJSON();
    expect(payload.generator).toBe('model');
    expect(payload.arguments).toEqual(['Post', 'title:string']);
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 0, stdout: 'Generated Post model', stderr: '' }),
    });
  });

  await page.route('**/__loco/cli/tasks/run', async (route) => {
    taskRunCalled = true;
    const payload = await route.request().postDataJSON();
    expect(payload.task).toBe('seed');
    expect(payload.arguments).toEqual(['alpha']);
    expect(payload.params).toEqual({ alpha: 'one' });
    await route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({ status: 1, stdout: 'Seed task', stderr: 'Seeding failed' }),
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

  const generatorForm = page.getByRole('form', { name: 'Generator command form' });
  await generatorForm.getByLabel('Generator').selectOption('model');
  await generatorForm.getByLabel('Arguments').fill('Post title:string');
  await generatorForm.getByRole('button', { name: /run generator/i }).click();
  await expect(page.getByText('Generated Post model')).toBeVisible();

  const taskForm = page.getByRole('form', { name: 'Task command form' });
  await taskForm.getByLabel('Task').selectOption('seed');
  await taskForm.getByLabel('Arguments').fill('alpha');
  await taskForm.getByLabel('Parameters').fill('alpha=one');
  await taskForm.getByRole('button', { name: /run task/i }).click();
  await expect(page.getByText('Seeding failed')).toBeVisible();

  expect(generatorRunCalled).toBeTruthy();
  expect(taskRunCalled).toBeTruthy();
});
