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
} from '../../core/models/CommandConsole';
import { CommandConsolePort } from '../../core/ports/CommandConsolePort';

interface CommandExecutionPayload {
  status: number;
  stdout: string;
  stderr: string;
}

interface DoctorSnapshotPayload {
  status: number;
  stdout: unknown;
  stderr: string;
}

interface JobStatusPayload {
  id: string;
  state: string;
  result?: CommandExecutionPayload;
  error?: string;
  updatedAt?: string;
}

const buildQuery = (params?: Record<string, string | undefined>): string => {
  if (!params) {
    return '';
  }
  const query = new URLSearchParams();
  Object.entries(params).forEach(([key, value]) => {
    if (value) {
      query.set(key, value);
    }
  });
  const queryString = query.toString();
  return queryString.length > 0 ? `?${queryString}` : '';
};

const parseCommandExecution = (payload: CommandExecutionPayload): CommandExecutionResult => ({
  status: payload.status,
  stdout: payload.stdout ?? '',
  stderr: payload.stderr ?? '',
});

const parseDoctorSnapshot = (payload: DoctorSnapshotPayload): DoctorSnapshotResult => ({
  status: payload.status,
  stdout: payload.stdout,
  stderr: payload.stderr ?? '',
});

const parseJobStatus = (payload: JobStatusPayload): CommandJobStatus => ({
  id: payload.id,
  state: payload.state,
  error: payload.error,
  updatedAt: payload.updatedAt,
  result: payload.result ? parseCommandExecution(payload.result) : undefined,
});

export class HttpCliService implements CommandConsolePort {
  constructor(private readonly baseUrl = '') {}

  async listGenerators(options?: ListCommandsOptions): Promise<ConsoleCommand[]> {
    const query = buildQuery({ environment: options?.environment });
    return this.get<ConsoleCommand[]>(`/__loco/cli/generators${query}`);
  }

  async runGenerator(request: RunGeneratorOptions): Promise<CommandExecutionResult> {
    const response = await this.post<CommandExecutionPayload>(
      '/__loco/cli/generators/run',
      {
        generator: request.generator,
        arguments: request.arguments ?? [],
        environment: request.environment,
      }
    );
    return parseCommandExecution(response);
  }

  async listTasks(options?: ListCommandsOptions): Promise<ConsoleCommand[]> {
    const query = buildQuery({ environment: options?.environment });
    return this.get<ConsoleCommand[]>(`/__loco/cli/tasks${query}`);
  }

  async runTask(request: RunTaskOptions): Promise<CommandExecutionResult> {
    const response = await this.post<CommandExecutionPayload>('/__loco/cli/tasks/run', {
      task: request.task,
      arguments: request.arguments ?? [],
      params: request.params ?? {},
      environment: request.environment,
    });
    return parseCommandExecution(response);
  }

  async requestDoctorSnapshot(options: DoctorSnapshotOptions): Promise<DoctorSnapshotResult> {
    const response = await this.post<DoctorSnapshotPayload>('/__loco/cli/doctor/snapshot', {
      environment: options.environment,
      production: options.production ?? false,
      config: options.config ?? false,
      graph: options.graph ?? false,
      assistant: options.assistant ?? false,
    });
    return parseDoctorSnapshot(response);
  }

  async fetchJobStatus(options: JobStatusOptions): Promise<CommandJobStatus> {
    const query = buildQuery({ environment: options.environment });
    const response = await this.get<JobStatusPayload>(
      `/__loco/cli/jobs/${encodeURIComponent(options.jobId)}${query}`
    );
    return parseJobStatus(response);
  }

  private async get<T>(path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      headers: { Accept: 'application/json' },
    });
    return this.parseJson<T>(response);
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify(body),
    });
    return this.parseJson<T>(response);
  }

  private async parseJson<T>(response: Response): Promise<T> {
    if (!response.ok) {
      let message = '';
      try {
        const data = await response.json();
        message = typeof data === 'string' ? data : JSON.stringify(data);
      } catch (err) {
        message = await response.text();
      }
      const errorMessage = message ? `${response.status} ${message}` : `HTTP ${response.status}`;
      throw new Error(`CLI request failed: ${errorMessage}`);
    }
    return (await response.json()) as T;
  }
}
