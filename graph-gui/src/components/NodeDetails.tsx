import { useEffect, useMemo, useState } from 'react';
import { AssistantState } from '../hooks/useGraph';
import { GraphNode } from '../core/models/Graph';

interface NodeDetailsProps {
  node: GraphNode | null;
  assistant: AssistantState;
  onRequestInsight: (node: GraphNode, prompt?: string) => void;
}

const formatType = (type: GraphNode['type']) =>
  type
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase());

export const NodeDetails = ({ node, assistant, onRequestInsight }: NodeDetailsProps) => {
  const [prompt, setPrompt] = useState('');

  useEffect(() => {
    setPrompt('');
  }, [node?.id]);

  const detailEntries = useMemo(() => {
    if (!node) {
      return [] as [string, string][];
    }

    switch (node.type) {
      case 'route':
        return [
          ['Path', node.data.path],
          ['Methods', node.data.methods.join(', ')],
        ];
      case 'background_worker':
        return [
          ['Command', node.data.command],
          ['Tags', (node.data.tags ?? []).join(', ') || '—'],
        ];
      case 'scheduler_job':
        return [
          ['Schedule', node.data.schedule],
          ['Command', node.data.command],
          ['Runs On Start', node.data.run_on_start ? 'Yes' : 'No'],
        ];
      case 'task':
        return [
          ['Name', node.data.name],
          ['Description', node.data.description ?? 'No description'],
        ];
      case 'application':
      default:
        return [
          ['Health', node.data.healthy ? 'Healthy' : 'Attention required'],
          ['Summary', node.data.description],
        ];
    }
  }, [node]);

  const handleRequest = () => {
    if (node) {
      onRequestInsight(node, prompt.trim() ? prompt : undefined);
    }
  };

  if (!node) {
    return (
      <div className="rounded-xl border border-dashed border-slate-700 bg-slate-900/40 p-6 text-center text-sm text-slate-400">
        Select a node to inspect its metadata and request AI guidance.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-4 rounded-xl border border-slate-800 bg-slate-900/60 p-4">
      <header className="flex items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold">{node.label}</h2>
          <p className="text-xs uppercase tracking-wide text-slate-400">
            {formatType(node.type)}
          </p>
        </div>
        <button
          type="button"
          onClick={handleRequest}
          className="rounded-lg bg-emerald-500 px-3 py-1 text-sm font-semibold text-slate-950 transition hover:bg-emerald-400 disabled:cursor-not-allowed disabled:opacity-60"
          disabled={assistant.status === 'loading'}
        >
          {assistant.status === 'loading' ? 'Requesting…' : 'Request AI guidance'}
        </button>
      </header>

      <dl className="grid grid-cols-1 gap-2 text-sm">
        {detailEntries.map(([title, value]) => (
          <div key={title}>
            <dt className="text-xs uppercase tracking-wide text-slate-500">{title}</dt>
            <dd className="text-slate-100">{value}</dd>
          </div>
        ))}
      </dl>

      <div className="flex flex-col gap-2">
        <label htmlFor="prompt" className="text-xs font-medium text-slate-400">
          Optional context for the assistant
        </label>
        <textarea
          id="prompt"
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
          rows={3}
          className="rounded-lg border border-slate-800 bg-slate-950 p-2 text-sm text-slate-100 focus:border-sky-500 focus:outline-none"
          placeholder="Provide extra context, e.g. error symptoms or expected behaviour"
        />
      </div>

      <section aria-live="polite" className="rounded-lg bg-slate-950/60 p-3 text-sm">
        {assistant.status === 'idle' && (
          <p className="text-slate-400">Use the assistant to obtain remediation tips for this component.</p>
        )}
        {assistant.status === 'loading' && (
          <p className="animate-pulse text-sky-400">Collecting insight…</p>
        )}
        {assistant.status === 'error' && assistant.error && (
          <p className="text-rose-400">{assistant.error}</p>
        )}
        {assistant.status === 'ready' && assistant.insight && (
          <div className="flex flex-col gap-3">
            <p className="font-semibold text-emerald-300">{assistant.insight.summary}</p>
            {assistant.insight.remediationTips.length > 0 ? (
              <ul className="list-disc space-y-1 pl-5 text-slate-200">
                {assistant.insight.remediationTips.map((tip, index) => (
                  <li key={index}>{tip}</li>
                ))}
              </ul>
            ) : (
              <p className="text-slate-400">No remediation tips provided.</p>
            )}
          </div>
        )}
      </section>
    </div>
  );
};
