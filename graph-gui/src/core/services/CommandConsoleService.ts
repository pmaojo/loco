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
} from '../models/CommandConsole';
import { CommandConsolePort } from '../ports/CommandConsolePort';

const trimOrUndefined = (value?: string): string | undefined => {
  if (!value) {
    return undefined;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
};

const sanitizeArguments = (argumentsList?: string[]): string[] => {
  if (!argumentsList) {
    return [];
  }
  return argumentsList.map((arg) => arg.trim()).filter((arg) => arg.length > 0);
};

const sanitizeParams = (params?: Record<string, string>): Record<string, string> | undefined => {
  if (!params) {
    return undefined;
  }
  const entries = Object.entries(params)
    .map(([key, value]) => [key.trim(), value.trim()] as const)
    .filter(([key, value]) => key.length > 0 && value.length > 0);
  if (entries.length === 0) {
    return undefined;
  }
  return Object.fromEntries(entries);
};

const sortCommands = (commands: ConsoleCommand[]): ConsoleCommand[] =>
  [...commands].sort((a, b) => a.command.localeCompare(b.command));

export class CommandConsoleService {
  constructor(private readonly port: CommandConsolePort) {}

  async listGenerators(options?: ListCommandsOptions): Promise<ConsoleCommand[]> {
    const environment = trimOrUndefined(options?.environment);
    const commands = await this.port.listGenerators({ environment });
    return sortCommands(commands);
  }

  async listTasks(options?: ListCommandsOptions): Promise<ConsoleCommand[]> {
    const environment = trimOrUndefined(options?.environment);
    const commands = await this.port.listTasks({ environment });
    return sortCommands(commands);
  }

  async runGenerator(options: RunGeneratorOptions): Promise<CommandExecutionResult> {
    const generator = options.generator.trim();
    if (generator.length === 0) {
      throw new Error('A generator name is required.');
    }
    const environment = trimOrUndefined(options.environment);
    const argumentsList = sanitizeArguments(options.arguments);
    return this.port.runGenerator({ generator, environment, arguments: argumentsList });
  }

  async runTask(options: RunTaskOptions): Promise<CommandExecutionResult> {
    const task = options.task.trim();
    if (task.length === 0) {
      throw new Error('A task name is required.');
    }
    const environment = trimOrUndefined(options.environment);
    const argumentsList = sanitizeArguments(options.arguments);
    const params = sanitizeParams(options.params);
    return this.port.runTask({ task, environment, arguments: argumentsList, params });
  }

  async requestDoctorSnapshot(options: DoctorSnapshotOptions): Promise<DoctorSnapshotResult> {
    const environment = trimOrUndefined(options.environment);
    const payload: DoctorSnapshotOptions = {
      environment,
      production: options.production ?? false,
      config: options.config ?? false,
      graph: options.graph ?? false,
      assistant: options.assistant ?? false,
    };
    return this.port.requestDoctorSnapshot(payload);
  }

  async fetchJobStatus(options: JobStatusOptions): Promise<CommandJobStatus> {
    const jobId = options.jobId.trim();
    if (jobId.length === 0) {
      throw new Error('A job identifier is required.');
    }
    const environment = trimOrUndefined(options.environment);
    return this.port.fetchJobStatus({ jobId, environment });
  }
}
