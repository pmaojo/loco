import { AssistantInsight, AssistantPort } from '../../core/models/Assistant';
import { GraphNode } from '../../core/models/Graph';

interface AssistantResponsePayload {
  summary?: string;
  remediationTips?: string[];
}

export class HttpAssistantService implements AssistantPort {
  constructor(private readonly baseUrl = '') {}

  async explainNode(
    node: GraphNode,
    options?: { prompt?: string }
  ): Promise<AssistantInsight> {
    const response = await fetch(`${this.baseUrl}/__loco/assistant`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        node: {
          id: node.id,
          label: node.label,
          type: node.type,
          data: node.data,
        },
        prompt: options?.prompt ?? null,
      }),
    });

    if (!response.ok) {
      throw new Error(`Unable to retrieve insights (status ${response.status})`);
    }

    const payload = (await response.json()) as AssistantResponsePayload;
    return {
      summary: payload.summary ?? 'The assistant returned no summary.',
      remediationTips: payload.remediationTips ?? [],
    };
  }
}
