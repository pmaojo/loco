import { FormEvent, useEffect, useMemo, useState } from 'react';
import classNames from 'classnames';
import { CommandConsoleService } from '../core/services/CommandConsoleService';
import { DoctorSnapshotOptions } from '../core/models/CommandConsole';
import { useCommandConsole } from '../hooks/useCommandConsole';
import { GeneratorForm } from './GeneratorForm';
import { CommandHistory } from './CommandHistory';

interface TaskFormProps {
  commands: { command: string; summary: string }[];
  loading?: boolean;
  error?: string;
  onSubmit: (options: {
    task: string;
    arguments: string[];
    params: Record<string, string>;
  }) => Promise<void> | void;
}

const parseTaskArguments = (value: string): string[] =>
  value
    .split(/\s+/)
    .map((item) => item.trim())
    .filter((item) => item.length > 0);

const parseTaskParams = (value: string): Record<string, string> => {
  const entries = value
    .split(/\r?\n|,/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => {
      const separator = line.includes('=') ? '=' : ':';
      const [key, ...rest] = line.split(separator);
      const normalizedKey = key.trim();
      const normalizedValue = rest.join(separator).trim();
      return [normalizedKey, normalizedValue] as const;
    })
    .filter(([key, value]) => key.length > 0 && value.length > 0);
  return Object.fromEntries(entries);
};

const TaskForm = ({ commands, loading, error, onSubmit }: TaskFormProps) => {
  const [selected, setSelected] = useState<string>(() => commands[0]?.command ?? '');
  const [args, setArgs] = useState<string>('');
  const [params, setParams] = useState<string>('');

  const selectedSummary = useMemo(
    () => commands.find((command) => command.command === selected)?.summary ?? '',
    [commands, selected]
  );

  const disabled = loading || commands.length === 0;

  useEffect(() => {
    setSelected((current) => {
      if (commands.length === 0) {
        return '';
      }
      if (commands.some((command) => command.command === current)) {
        return current;
      }
      return commands[0].command;
    });
  }, [commands]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selected) {
      return;
    }
    await onSubmit({
      task: selected,
      arguments: parseTaskArguments(args),
      params: parseTaskParams(params),
    });
    setArgs('');
    setParams('');
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4" aria-label="Task command form">
      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="task-select">
          Task
        </label>
        <select
          id="task-select"
          className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
          value={selected}
          onChange={(event) => setSelected(event.target.value)}
          disabled={disabled}
        >
          {commands.map((command) => (
            <option key={command.command} value={command.command}>
              {command.command}
            </option>
          ))}
        </select>
        {selectedSummary ? <p className="text-xs text-slate-400">{selectedSummary}</p> : null}
        {commands.length === 0 ? (
          <p className="text-xs text-slate-500">No tasks available for this environment.</p>
        ) : null}
      </div>

      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="task-arguments">
          Arguments
        </label>
        <input
          id="task-arguments"
          type="text"
          placeholder="user:admin"
          className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
          value={args}
          onChange={(event) => setArgs(event.target.value)}
          disabled={disabled}
        />
        <p className="text-xs text-slate-500">Separate arguments with spaces.</p>
      </div>

      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="task-params">
          Parameters
        </label>
        <textarea
          id="task-params"
          rows={3}
          placeholder={`alpha=one\nbeta=two`}
          className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
          value={params}
          onChange={(event) => setParams(event.target.value)}
          disabled={disabled}
        />
        <p className="text-xs text-slate-500">One key=value pair per line. Colons are also supported.</p>
      </div>

      {error ? <p className="text-sm text-rose-400">{error}</p> : null}

      <button
        type="submit"
        className={classNames(
          'rounded-lg bg-sky-500 px-4 py-2 text-sm font-semibold text-sky-950 transition hover:bg-sky-400',
          { 'opacity-50': disabled }
        )}
        disabled={disabled}
      >
        {loading ? 'Running…' : 'Run task'}
      </button>
    </form>
  );
};

const defaultDoctorOptions: DoctorSnapshotOptions = {
  production: false,
  config: false,
  graph: false,
  assistant: false,
};

const DoctorForm = ({
  loading,
  error,
  onSubmit,
}: {
  loading?: boolean;
  error?: string;
  onSubmit: (options: DoctorSnapshotOptions) => Promise<void> | void;
}) => {
  const [options, setOptions] = useState<DoctorSnapshotOptions>(defaultDoctorOptions);

  const toggle = (key: keyof DoctorSnapshotOptions) => {
    setOptions((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onSubmit(options);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-4" aria-label="Doctor command form">
      <fieldset className="space-y-2">
        <legend className="text-sm font-medium text-slate-300">Snapshot options</legend>
        <div className="grid gap-2 sm:grid-cols-2">
          {(['production', 'config', 'graph', 'assistant'] as (keyof DoctorSnapshotOptions)[]).map((key) => (
            <label key={key} className="flex items-center gap-2 text-xs text-slate-300">
              <input
                type="checkbox"
                checked={Boolean(options[key])}
                onChange={() => toggle(key)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900 text-emerald-500 focus:ring-emerald-500"
              />
              <span className="capitalize">{key}</span>
            </label>
          ))}
        </div>
      </fieldset>

      {error ? <p className="text-sm text-rose-400">{error}</p> : null}

      <button
        type="submit"
        className={classNames(
          'rounded-lg bg-purple-500 px-4 py-2 text-sm font-semibold text-purple-950 transition hover:bg-purple-400',
          { 'opacity-50': loading }
        )}
        disabled={loading}
      >
        {loading ? 'Requesting…' : 'Run doctor snapshot'}
      </button>
    </form>
  );
};

const JobStatusForm = ({
  loading,
  error,
  onSubmit,
}: {
  loading?: boolean;
  error?: string;
  onSubmit: (jobId: string) => Promise<void> | void;
}) => {
  const [jobId, setJobId] = useState('');

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    await onSubmit(jobId);
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-3" aria-label="Job status form">
      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="job-identifier">
          Job identifier
        </label>
        <input
          id="job-identifier"
          type="text"
          value={jobId}
          onChange={(event) => setJobId(event.target.value)}
          className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
          placeholder="job-123"
        />
      </div>
      {error ? <p className="text-sm text-rose-400">{error}</p> : null}
      <button
        type="submit"
        className={classNames(
          'rounded-lg bg-amber-500 px-4 py-2 text-sm font-semibold text-amber-950 transition hover:bg-amber-400',
          { 'opacity-50': loading }
        )}
        disabled={loading}
      >
        {loading ? 'Fetching…' : 'Fetch job status'}
      </button>
    </form>
  );
};

export const CommandConsole = ({ service }: { service: CommandConsoleService }) => {
  const {
    loadStatus,
    loadError,
    generators,
    tasks,
    generatorStatus,
    generatorError,
    taskStatus,
    taskError,
    doctorStatus,
    doctorError,
    jobStatus,
    jobError,
    history,
    environment,
    setEnvironment,
    runGenerator,
    runTask,
    requestDoctor,
    fetchJobStatus,
    reloadCommands,
  } = useCommandConsole(service);

  const isLoadingCommands = loadStatus === 'loading';

  return (
    <section className="rounded-2xl border border-slate-800 bg-slate-950/50 p-6 shadow-lg shadow-slate-950/40">
      <header className="mb-6 space-y-2">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-xl font-semibold text-slate-100">Command Console</h2>
            <p className="text-sm text-slate-400">
              Run approved generators, tasks and diagnostics directly from the browser.
            </p>
          </div>
          <button
            type="button"
            onClick={() => reloadCommands()}
            className="rounded-md border border-slate-700 px-3 py-1 text-xs font-semibold text-slate-300 transition hover:border-slate-500 hover:text-slate-100"
            disabled={isLoadingCommands}
          >
            Refresh commands
          </button>
        </div>
        <div className="space-y-2">
          <label className="block text-xs font-medium uppercase tracking-wider text-slate-400" htmlFor="console-environment">
            Environment
          </label>
          <input
            id="console-environment"
            type="text"
            placeholder="development"
            className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
            value={environment ?? ''}
            onChange={(event) => setEnvironment(event.target.value)}
          />
        </div>
      </header>

      {loadStatus === 'error' ? (
        <div className="mb-6 rounded-xl border border-rose-700 bg-rose-950/30 p-4 text-sm text-rose-300">
          <p className="mb-2">Unable to load commands: {loadError}</p>
          <button
            type="button"
            onClick={() => reloadCommands()}
            className="rounded-md bg-rose-500 px-3 py-1 text-xs font-semibold text-rose-950 transition hover:bg-rose-400"
          >
            Retry
          </button>
        </div>
      ) : null}

      <div className="grid gap-8 lg:grid-cols-[1fr_1fr]">
        <div className="space-y-8">
          <div className="rounded-xl border border-slate-800 bg-slate-950/40 p-5">
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wide text-slate-300">Generators</h3>
            <GeneratorForm
              commands={generators}
              loading={generatorStatus === 'running' || isLoadingCommands}
              error={generatorError}
              onSubmit={runGenerator}
            />
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/40 p-5">
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wide text-slate-300">Tasks</h3>
            <TaskForm
              commands={tasks}
              loading={taskStatus === 'running' || isLoadingCommands}
              error={taskError}
              onSubmit={runTask}
            />
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/40 p-5">
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wide text-slate-300">Doctor</h3>
            <DoctorForm
              loading={doctorStatus === 'running'}
              error={doctorError}
              onSubmit={requestDoctor}
            />
          </div>

          <div className="rounded-xl border border-slate-800 bg-slate-950/40 p-5">
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wide text-slate-300">Job status</h3>
            <JobStatusForm
              loading={jobStatus === 'running'}
              error={jobError}
              onSubmit={fetchJobStatus}
            />
          </div>
        </div>

        <div>
          <h3 className="mb-4 text-sm font-semibold uppercase tracking-wide text-slate-300">History</h3>
          {isLoadingCommands && history.length === 0 ? (
            <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-4 text-sm text-slate-400">
              Loading commands…
            </div>
          ) : (
            <CommandHistory
              history={history}
              jobLoading={jobStatus === 'running'}
              onRefreshJob={fetchJobStatus}
            />
          )}
        </div>
      </div>
    </section>
  );
};
