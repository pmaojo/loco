import { expect, test } from '@playwright/test';

test.describe('smoke', () => {
  test('runs generators, tasks and doctor snapshot against mocked backend', async ({ page }) => {
    let generatorPayload: any;
    let taskPayload: any;
    let doctorPayload: any;

    await page.route('**/__loco/graph', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          routes: [
            { path: '/__loco/graph', methods: ['GET'] },
            { path: '/__loco/cli/generators', methods: ['GET'] },
          ],
          dependencies: {
            background_workers: [],
            scheduler_jobs: [],
            tasks: [],
          },
          health: { ok: true },
        }),
      });
    });

    await page.route('**/__loco/cli/generators', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { command: 'model', summary: 'Generate a model' },
          { command: 'migration', summary: 'Generate a migration' },
        ]),
      });
    });

    await page.route('**/__loco/cli/tasks', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { command: 'seed', summary: 'Seed the database' },
          { command: 'refresh', summary: 'Refresh projections' },
        ]),
      });
    });

    await page.route('**/__loco/cli/generators/run', async (route) => {
      generatorPayload = await route.request().postDataJSON();
      expect(generatorPayload.generator).toBe('model');
      expect(generatorPayload.arguments).toEqual(['User', 'email:string']);
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 0, stdout: 'Generated user model', stderr: '' }),
      });
    });

    await page.route('**/__loco/cli/tasks/run', async (route) => {
      taskPayload = await route.request().postDataJSON();
      expect(taskPayload.task).toBe('seed');
      expect(taskPayload.params).toEqual({ alpha: 'one', beta: 'two' });
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({ status: 0, stdout: 'Seed fixtures executed', stderr: '' }),
      });
    });

    await page.route('**/__loco/cli/doctor/snapshot', async (route) => {
      doctorPayload = await route.request().postDataJSON();
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          status: 0,
          stdout: { ok: true, findings: [{ name: 'db', status: 'passing' }] },
          stderr: '',
        }),
      });
    });

    await page.goto('/');

    await expect(page.getByRole('heading', { name: 'Command Console' })).toBeVisible();
    await page.getByLabel('Environment').fill('qa');

    const generatorForm = page.getByRole('form', { name: 'Generator command form' });
    await generatorForm.getByLabel('Generator').selectOption('model');
    await generatorForm.getByLabel('Arguments').fill('User email:string');
    await generatorForm.getByRole('button', { name: /run generator/i }).click();

    await expect(page.getByText('Generated user model')).toBeVisible();

    const taskForm = page.getByRole('form', { name: 'Task command form' });
    await taskForm.getByLabel('Task').selectOption('seed');
    await taskForm.getByLabel('Arguments').fill('alpha');
    await taskForm.getByLabel('Parameters').fill('alpha=one\nbeta=two');
    await taskForm.getByRole('button', { name: /run task/i }).click();

    await expect(page.getByText('Seed fixtures executed')).toBeVisible();

    const doctorForm = page.getByRole('form', { name: 'Doctor command form' });
    await doctorForm.getByLabel('Production').check();
    await doctorForm.getByLabel('Config').check();
    await doctorForm.getByLabel('Graph').check();
    await doctorForm.getByLabel('Assistant').check();
    await doctorForm.getByRole('button', { name: /run doctor snapshot/i }).click();

    await expect(page.getByText('"status": "passing"')).toBeVisible();

    await expect(page.locator('[data-testid^="history-"]')).toHaveCount(3);

    expect(generatorPayload.environment).toBe('qa');
    expect(taskPayload.environment).toBe('qa');
    expect(taskPayload.arguments).toEqual(['alpha']);
    expect(doctorPayload.environment).toBe('qa');
    expect(doctorPayload.production).toBe(true);
    expect(doctorPayload.config).toBe(true);
    expect(doctorPayload.graph).toBe(true);
    expect(doctorPayload.assistant).toBe(true);
  });
});
