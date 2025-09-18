export interface ConsoleCommand {
  command: string;
  summary: string;
}

export interface CommandExecutionResult {
  status: number;
  stdout: string;
  stderr: string;
}

export interface DoctorSnapshotResult {
  status: number;
  stdout: unknown;
  stderr: string;
}

export interface CommandJobStatus {
  id: string;
  state: string;
  result?: CommandExecutionResult;
  error?: string;
  updatedAt?: string;
}

export interface ListCommandsOptions {
  environment?: string;
}

export interface RunGeneratorOptions {
  generator: string;
  arguments?: string[];
  environment?: string;
}

export interface RunTaskOptions {
  task: string;
  arguments?: string[];
  params?: Record<string, string>;
  environment?: string;
}

export interface DoctorSnapshotOptions {
  environment?: string;
  production?: boolean;
  config?: boolean;
  graph?: boolean;
  assistant?: boolean;
}

export interface JobStatusOptions {
  jobId: string;
  environment?: string;
}

export type CommandKind = 'generator' | 'task' | 'doctor' | 'job';

export type HistoryStatus = 'running' | 'success' | 'error';

export interface CommandHistoryEntry {
  id: string;
  kind: CommandKind;
  command: string;
  status: HistoryStatus;
  startedAt: number;
  completedAt?: number;
  environment?: string;
  exitCode?: number;
  stdout?: unknown;
  stderr?: string;
  errorMessage?: string;
  context?: {
    jobId?: string;
  };
}
