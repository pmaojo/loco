import { describe, expect, it, vi } from 'vitest';
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

const createPort = () => {
  return {
    listGenerators: vi.fn<[ListCommandsOptions?], Promise<ConsoleCommand[]>>().mockResolvedValue([]),
    listTasks: vi.fn<[ListCommandsOptions?], Promise<ConsoleCommand[]>>().mockResolvedValue([]),
    runGenerator: vi.fn<[RunGeneratorOptions], Promise<CommandExecutionResult>>().mockResolvedValue({
      status: 0,
      stdout: 'ok',
      stderr: '',
    }),
    runTask: vi.fn<[RunTaskOptions], Promise<CommandExecutionResult>>().mockResolvedValue({
      status: 0,
      stdout: 'ok',
      stderr: '',
    }),
    requestDoctorSnapshot: vi.fn<[DoctorSnapshotOptions], Promise<DoctorSnapshotResult>>().mockResolvedValue({
      status: 0,
      stdout: { ok: true },
      stderr: '',
    }),
    fetchJobStatus: vi.fn<[JobStatusOptions], Promise<CommandJobStatus>>().mockResolvedValue({
      id: 'job-1',
      state: 'completed',
      result: { status: 0, stdout: 'done', stderr: '' },
    }),
  } satisfies CommandConsolePort;
};

describe('CommandConsoleService', () => {
  it('sorts generators alphabetically and normalizes environment', async () => {
    const port = createPort();
    port.listGenerators.mockResolvedValueOnce([
      { command: 'zulu', summary: 'later' },
      { command: 'alpha', summary: 'first' },
    ]);
    const service = new CommandConsoleService(port);

    const commands = await service.listGenerators({ environment: '  dev  ' });

    expect(commands.map((command) => command.command)).toEqual(['alpha', 'zulu']);
    expect(port.listGenerators).toHaveBeenCalledWith({ environment: 'dev' });
  });

  it('sanitizes generator input before delegating to the port', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);

    await service.runGenerator({ generator: '  model  ', arguments: [' posts ', ' '] });

    expect(port.runGenerator).toHaveBeenCalledWith({
      generator: 'model',
      environment: undefined,
      arguments: ['posts'],
    } satisfies RunGeneratorOptions);
  });

  it('normalizes task arguments and params', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);

    await service.runTask({
      task: '  seed  ',
      arguments: [' users ', ''],
      params: { ' alpha ': ' one ', beta: '' },
      environment: ' staging ',
    });

    expect(port.runTask).toHaveBeenCalledWith({
      task: 'seed',
      arguments: ['users'],
      params: { alpha: 'one' },
      environment: 'staging',
    } satisfies RunTaskOptions);
  });

  it('propagates boolean defaults for doctor snapshot', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);

    await service.requestDoctorSnapshot({ environment: ' prod ' });

    expect(port.requestDoctorSnapshot).toHaveBeenCalledWith({
      environment: 'prod',
      production: false,
      config: false,
      graph: false,
      assistant: false,
    } satisfies DoctorSnapshotOptions);
  });

  it('trims job identifiers before fetching status', async () => {
    const port = createPort();
    const service = new CommandConsoleService(port);

    await service.fetchJobStatus({ jobId: ' job-42 ', environment: ' qa ' });

    expect(port.fetchJobStatus).toHaveBeenCalledWith({ jobId: 'job-42', environment: 'qa' });
  });
});
