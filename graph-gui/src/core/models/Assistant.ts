import { GraphNode } from './Graph';

export interface AssistantInsight {
  summary: string;
  remediationTips: string[];
}

export interface AssistantRequest {
  node: GraphNode;
  prompt?: string;
}

export interface AssistantPort {
  explainNode(node: GraphNode, options?: { prompt?: string }): Promise<AssistantInsight>;
}
