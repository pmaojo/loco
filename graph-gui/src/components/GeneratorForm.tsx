import { FormEvent, useEffect, useMemo, useState } from 'react';
import classNames from 'classnames';

export interface GeneratorFormProps {
  commands: { command: string; summary: string }[];
  loading?: boolean;
  error?: string;
  onSubmit: (options: { generator: string; arguments: string[] }) => Promise<void> | void;
}

const parseArguments = (input: string): string[] =>
  input
    .split(/\s+/)
    .map((value) => value.trim())
    .filter((value) => value.length > 0);

export const GeneratorForm = ({ commands, loading, error, onSubmit }: GeneratorFormProps) => {
  const [selected, setSelected] = useState<string>(() => commands[0]?.command ?? '');
  const [args, setArgs] = useState<string>('');

  useEffect(() => {
    if (commands.length === 0) {
      if (selected !== '') {
        setSelected('');
      }
      return;
    }

    if (!commands.some((command) => command.command === selected)) {
      setSelected(commands[0].command);
    }
  }, [commands, selected]);

  const summary = useMemo(() => {
    return commands.find((command) => command.command === selected)?.summary ?? '';
  }, [commands, selected]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (!selected) {
      return;
    }
    const argumentsList = parseArguments(args);
    await onSubmit({ generator: selected, arguments: argumentsList });
    setArgs('');
  };

  const disabled = loading || commands.length === 0;

  return (
    <form onSubmit={handleSubmit} className="space-y-4" aria-label="Generator command form">
      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="generator-select">
          Generator
        </label>
        <select
          id="generator-select"
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
        {summary ? <p className="text-xs text-slate-400">{summary}</p> : null}
        {commands.length === 0 ? (
          <p className="text-xs text-slate-500">No generators available for this environment.</p>
        ) : null}
      </div>

      <div className="space-y-2">
        <label className="block text-sm font-medium text-slate-300" htmlFor="generator-arguments">
          Arguments
        </label>
        <input
          id="generator-arguments"
          type="text"
          placeholder="e.g. Post title:string"
          className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3 py-2 text-sm text-slate-100 focus:border-slate-500 focus:outline-none"
          value={args}
          onChange={(event) => setArgs(event.target.value)}
          disabled={disabled}
        />
        <p className="text-xs text-slate-500">Separate arguments with spaces.</p>
      </div>

      {error ? <p className="text-sm text-rose-400">{error}</p> : null}

      <div className="flex items-center gap-3">
        <button
          type="submit"
          className={classNames(
            'rounded-lg bg-emerald-500 px-4 py-2 text-sm font-semibold text-emerald-950 transition hover:bg-emerald-400',
            { 'opacity-50': disabled }
          )}
          disabled={disabled}
        >
          {loading ? 'Running…' : 'Run generator'}
        </button>
      </div>
    </form>
  );
};
