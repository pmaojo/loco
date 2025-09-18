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

export interface CommandConsolePort {
  listGenerators(options?: ListCommandsOptions): Promise<ConsoleCommand[]>;
  runGenerator(request: RunGeneratorOptions): Promise<CommandExecutionResult>;
  listTasks(options?: ListCommandsOptions): Promise<ConsoleCommand[]>;
  runTask(request: RunTaskOptions): Promise<CommandExecutionResult>;
  requestDoctorSnapshot(options: DoctorSnapshotOptions): Promise<DoctorSnapshotResult>;
  fetchJobStatus(options: JobStatusOptions): Promise<CommandJobStatus>;
}
