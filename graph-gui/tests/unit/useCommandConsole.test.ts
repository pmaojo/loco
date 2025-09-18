import { describe, expect, it, vi } from 'vitest';
import { createCommandConsoleStore } from '../../src/hooks/useCommandConsole';
import { CommandConsoleService } from '../../src/core/services/CommandConsoleService';
import { CommandConsolePort } from '../../src/core/ports/CommandConsolePort';
import {
  CommandExecutionResult,
  CommandJobStatus,
  ConsoleCommand,
  DoctorSnapshotOptions,
  DoctorSnapshotResult,
  JobStatusOptions,
  ListCommandsOptions,
  RunGeneratorOptions,
  RunTaskOptions,
} from '../../src/core/models/CommandConsole';

const createPort = () => ({
  listGenerators: vi
    .fn<[ListCommandsOptions?], Promise<ConsoleCommand[]>>()
    .mockResolvedValue([{ command: 'model', summary: 'Create model' }]),
  listTasks: vi
    .fn<[ListCommandsOptions?], Promise<ConsoleCommand[]>>()
    .mockResolvedValue([{ command: 'seed', summary: 'Seed data' }]),
  runGenerator: vi.fn<[RunGeneratorOptions], Promise<CommandExecutionResult>>().mockResolvedValue({
    status: 0,
    stdout: 'generated',
    stderr: '',
  }),
  runTask: vi.fn<[RunTaskOptions], Promise<CommandExecutionResult>>().mockResolvedValue({
    status: 1,
    stdout: 'failed',
    stderr: 'boom',
  }),
  requestDoctorSnapshot: vi
    .fn<[DoctorSnapshotOptions], Promise<DoctorSnapshotResult>>()
    .mockResolvedValue({ status: 0, stdout: { ok: true }, stderr: '' }),
  fetchJobStatus: vi.fn<[JobStatusOptions], Promise<CommandJobStatus>>().mockResolvedValue({
    id: 'job-1',
    state: 'completed',
    result: { status: 0, stdout: 'done', stderr: '' },
  }),
}) satisfies CommandConsolePort;

describe('useCommandConsole store', () => {
  it('records generator executions in history', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);
    const store = createCommandConsoleStore(service);

    await store.getState().runGenerator({ generator: 'model', arguments: ['post'] });

    const history = store.getState().history;
    expect(port.runGenerator).toHaveBeenCalledWith({
      generator: 'model',
      arguments: ['post'],
      environment: undefined,
    } satisfies RunGeneratorOptions);
    expect(history).toHaveLength(1);
    expect(history[0].status).toBe('success');
    expect(history[0].stdout).toBe('generated');
  });

  it('captures failing task executions', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);
    const store = createCommandConsoleStore(service);

    await store.getState().runTask({ task: 'seed', arguments: [], params: {} });

    const [entry] = store.getState().history;
    expect(entry.stderr).toBe('boom');
    expect(entry.status).toBe('error');
    expect(entry.exitCode).toBe(1);
  });

  it('updates job status entries', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);
    const store = createCommandConsoleStore(service);

    await store.getState().fetchJobStatus('job-1');

    expect(port.fetchJobStatus).toHaveBeenCalledWith({ jobId: 'job-1', environment: undefined });
    const [entry] = store.getState().history;
    expect(entry.kind).toBe('job');
    expect(entry.context?.jobId).toBe('job-1');
    expect(entry.status).toBe('success');
  });
});
