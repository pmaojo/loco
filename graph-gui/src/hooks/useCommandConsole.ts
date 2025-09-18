import { useEffect, useRef } from 'react';
import { StoreApi, createStore } from 'zustand/vanilla';
import { useStore } from 'zustand';
import {
  CommandHistoryEntry,
  CommandJobStatus,
  DoctorSnapshotOptions,
  HistoryStatus,
} from '../core/models/CommandConsole';
import { CommandConsoleService } from '../core/services/CommandConsoleService';

const MAX_HISTORY = 20;

type LoadStatus = 'idle' | 'loading' | 'ready' | 'error';
type OperationStatus = 'idle' | 'running';

type CommandOperation = 'generator' | 'task' | 'doctor' | 'job';

interface CommandConsoleState {
  loadStatus: LoadStatus;
  loadError?: string;
  environment?: string;
  generators: { command: string; summary: string }[];
  tasks: { command: string; summary: string }[];
  history: CommandHistoryEntry[];
  generatorStatus: OperationStatus;
  generatorError?: string;
  taskStatus: OperationStatus;
  taskError?: string;
  doctorStatus: OperationStatus;
  doctorError?: string;
  jobStatus: OperationStatus;
  jobError?: string;
  initialize: (environment?: string) => Promise<void>;
  reloadCommands: () => Promise<void>;
  setEnvironment: (environment: string) => void;
  runGenerator: (options: { generator: string; arguments: string[] }) => Promise<void>;
  runTask: (options: { task: string; arguments: string[]; params: Record<string, string> }) => Promise<void>;
  requestDoctor: (options: DoctorSnapshotOptions) => Promise<void>;
  fetchJobStatus: (jobId: string) => Promise<void>;
}

const randomId = (): string => {
  const globalCrypto = globalThis.crypto as { randomUUID?: () => string } | undefined;
  if (globalCrypto?.randomUUID) {
    return globalCrypto.randomUUID();
  }
  return `cmd-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

const sanitizeArguments = (values: string[]): string[] =>
  values.map((value) => value.trim()).filter((value) => value.length > 0);

const sanitizeParams = (params: Record<string, string>): Record<string, string> => {
  const entries = Object.entries(params)
    .map(([key, value]) => [key.trim(), value.trim()] as const)
    .filter(([key, value]) => key.length > 0 && value.length > 0);
  return Object.fromEntries(entries);
};

const withEnvironment = (parts: string[], environment?: string): string => {
  if (!environment) {
    return parts.join(' ');
  }
  return `${parts.join(' ')} --environment ${environment}`;
};

const appendHistory = (history: CommandHistoryEntry[], entry: CommandHistoryEntry): CommandHistoryEntry[] => {
  const next = [entry, ...history];
  return next.slice(0, MAX_HISTORY);
};

const updateHistoryEntry = (
  history: CommandHistoryEntry[],
  id: string,
  updates: Partial<CommandHistoryEntry>
): CommandHistoryEntry[] => history.map((entry) => (entry.id === id ? { ...entry, ...updates } : entry));

const evaluateExitCode = (exitCode: number | undefined, error?: string): HistoryStatus => {
  if (typeof exitCode !== 'number') {
    return error ? 'error' : 'success';
  }
  return exitCode === 0 && !error ? 'success' : 'error';
};

const deriveJobHistory = (status: CommandJobStatus): {
  status: HistoryStatus;
  exitCode?: number;
  stdout?: unknown;
  stderr?: string;
  errorMessage?: string;
} => {
  if (!status.result) {
    if (status.error) {
      return { status: 'error', errorMessage: status.error, stderr: status.error };
    }
    return { status: 'running', stdout: status.state };
  }
  const { stdout, stderr, status: exitCode } = status.result;
  const historyStatus = evaluateExitCode(exitCode, status.error || stderr);
  return {
    status: historyStatus,
    exitCode,
    stdout: stdout ?? status.state,
    stderr,
    errorMessage: historyStatus === 'error' ? status.error ?? stderr : undefined,
  };
};

const setOperationStatus = (
  operation: CommandOperation,
  status: OperationStatus,
  set: StoreApi<CommandConsoleState>['setState']
) => {
  const key = `${operation}Status` as const;
  set({ [key]: status } as Partial<CommandConsoleState>);
};

const clearOperationError = (
  operation: CommandOperation,
  set: StoreApi<CommandConsoleState>['setState']
) => {
  const key = `${operation}Error` as const;
  set({ [key]: undefined } as Partial<CommandConsoleState>);
};

const assignOperationError = (
  operation: CommandOperation,
  error: string,
  set: StoreApi<CommandConsoleState>['setState']
) => {
  const key = `${operation}Error` as const;
  set({ [key]: error } as Partial<CommandConsoleState>);
};

export const createCommandConsoleStore = (
  service: CommandConsoleService
): StoreApi<CommandConsoleState> =>
  createStore<CommandConsoleState>((set, get) => ({
    loadStatus: 'idle',
    generators: [],
    tasks: [],
    history: [],
    generatorStatus: 'idle',
    taskStatus: 'idle',
    doctorStatus: 'idle',
    jobStatus: 'idle',
    initialize: async (environment?: string) => {
      set({ loadStatus: 'loading', loadError: undefined });
      const effectiveEnv = environment ?? get().environment;
      try {
        const [generators, tasks] = await Promise.all([
          service.listGenerators({ environment: effectiveEnv }),
          service.listTasks({ environment: effectiveEnv }),
        ]);
        set({
          loadStatus: 'ready',
          generators,
          tasks,
          environment: effectiveEnv,
        });
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        set({
          loadStatus: 'error',
          loadError: message,
          generators: [],
          tasks: [],
        });
      }
    },
    reloadCommands: async () => {
      const env = get().environment;
      await get().initialize(env);
    },
    setEnvironment: (environment: string) => {
      set({ environment });
    },
    runGenerator: async ({ generator, arguments: args }) => {
      const environment = get().environment;
      const argumentsList = sanitizeArguments(args);
      const command = withEnvironment(
        ['cargo', 'loco', 'generate', generator, ...argumentsList],
        environment
      );
      const entryId = randomId();
      const startedAt = Date.now();
      set((state) => ({
        history: appendHistory(state.history, {
          id: entryId,
          kind: 'generator',
          command,
          status: 'running',
          startedAt,
          environment,
        }),
      }));
      setOperationStatus('generator', 'running', set);
      clearOperationError('generator', set);
      try {
        const result = await service.runGenerator({
          generator,
          arguments: argumentsList,
          environment,
        });
        const exitCode = result.status;
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: evaluateExitCode(exitCode, result.stderr),
            exitCode,
            stdout: result.stdout,
            stderr: result.stderr,
            completedAt: Date.now(),
          }),
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        assignOperationError('generator', message, set);
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: 'error',
            errorMessage: message,
            completedAt: Date.now(),
          }),
        }));
      } finally {
        setOperationStatus('generator', 'idle', set);
      }
    },
    runTask: async ({ task, arguments: args, params }) => {
      const environment = get().environment;
      const argumentsList = sanitizeArguments(args);
      const sanitizedParams = sanitizeParams(params);
      const paramPairs = Object.entries(sanitizedParams).map(
        ([key, value]) => `${key}:${value}`
      );
      const command = withEnvironment(
        ['cargo', 'loco', 'task', task, ...argumentsList, ...paramPairs],
        environment
      );
      const entryId = randomId();
      const startedAt = Date.now();
      set((state) => ({
        history: appendHistory(state.history, {
          id: entryId,
          kind: 'task',
          command,
          status: 'running',
          startedAt,
          environment,
        }),
      }));
      setOperationStatus('task', 'running', set);
      clearOperationError('task', set);
      try {
        const result = await service.runTask({
          task,
          arguments: argumentsList,
          params: sanitizedParams,
          environment,
        });
        const exitCode = result.status;
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: evaluateExitCode(exitCode, result.stderr),
            exitCode,
            stdout: result.stdout,
            stderr: result.stderr,
            completedAt: Date.now(),
          }),
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        assignOperationError('task', message, set);
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: 'error',
            errorMessage: message,
            completedAt: Date.now(),
          }),
        }));
      } finally {
        setOperationStatus('task', 'idle', set);
      }
    },
    requestDoctor: async (options: DoctorSnapshotOptions) => {
      const environment = get().environment;
      const command = withEnvironment(
        [
          'cargo',
          'loco',
          'doctor',
          options.production ? '--production' : undefined,
          options.config ? '--config' : undefined,
          options.graph ? '--graph' : undefined,
          options.assistant ? '--assistant' : undefined,
        ].filter((part): part is string => Boolean(part)),
        environment
      );
      const entryId = randomId();
      const startedAt = Date.now();
      set((state) => ({
        history: appendHistory(state.history, {
          id: entryId,
          kind: 'doctor',
          command,
          status: 'running',
          startedAt,
          environment,
        }),
      }));
      setOperationStatus('doctor', 'running', set);
      clearOperationError('doctor', set);
      try {
        const result = await service.requestDoctorSnapshot({ ...options, environment });
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: evaluateExitCode(result.status, result.stderr),
            exitCode: result.status,
            stdout: result.stdout,
            stderr: result.stderr,
            completedAt: Date.now(),
          }),
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        assignOperationError('doctor', message, set);
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: 'error',
            errorMessage: message,
            completedAt: Date.now(),
          }),
        }));
      } finally {
        setOperationStatus('doctor', 'idle', set);
      }
    },
    fetchJobStatus: async (jobId: string) => {
      const trimmedJobId = jobId.trim();
      if (trimmedJobId.length === 0) {
        assignOperationError('job', 'A job identifier is required.', set);
        return;
      }
      const environment = get().environment;
      const command = withEnvironment(
        ['cargo', 'loco', 'jobs', 'status', trimmedJobId],
        environment
      );
      const history = get().history;
      const existing = history.find((entry) => entry.kind === 'job' && entry.command === command);
      const entryId = existing?.id ?? randomId();
      const startedAt = existing?.startedAt ?? Date.now();
      const updatedHistory = existing
        ? updateHistoryEntry(history, entryId, {
            status: 'running',
            completedAt: undefined,
            exitCode: undefined,
            errorMessage: undefined,
            stderr: undefined,
          })
        : appendHistory(history, {
            id: entryId,
            kind: 'job',
            command,
            status: 'running',
            startedAt,
            environment,
            context: { jobId: trimmedJobId },
          });
      set({ history: updatedHistory });
      setOperationStatus('job', 'running', set);
      clearOperationError('job', set);
      try {
        const status = await service.fetchJobStatus({ jobId: trimmedJobId, environment });
        const details = deriveJobHistory(status);
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: details.status,
            exitCode: details.exitCode,
            stdout: details.stdout,
            stderr: details.stderr,
            errorMessage: details.errorMessage,
            completedAt: details.status === 'running' ? undefined : Date.now(),
          }),
        }));
      } catch (error) {
        const message = error instanceof Error ? error.message : 'Unknown error';
        assignOperationError('job', message, set);
        set((state) => ({
          history: updateHistoryEntry(state.history, entryId, {
            status: 'error',
            errorMessage: message,
            completedAt: Date.now(),
          }),
        }));
      } finally {
        setOperationStatus('job', 'idle', set);
      }
    },
  }));

export const useCommandConsole = (service: CommandConsoleService): CommandConsoleState => {
  const storeRef = useRef<StoreApi<CommandConsoleState>>();
  if (!storeRef.current) {
    storeRef.current = createCommandConsoleStore(service);
  }
  const state = useStore(storeRef.current);
  useEffect(() => {
    void storeRef.current?.getState().initialize();
  }, []);
  return state;
};
