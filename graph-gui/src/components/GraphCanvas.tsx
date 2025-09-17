import { useEffect, useMemo, useState } from 'react';
import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  SimulationLinkDatum,
  SimulationNodeDatum,
} from 'd3-force';
import classNames from 'classnames';
import { GraphEdge, GraphNode } from '../core/models/Graph';

interface GraphCanvasProps {
  nodes: GraphNode[];
  edges: GraphEdge[];
  onNodeSelect: (node: GraphNode) => void;
  selectedNode: GraphNode | null;
}

type PositionedNode = GraphNode & SimulationNodeDatum & { x: number; y: number };
type PositionedLink = SimulationLinkDatum<PositionedNode> & GraphEdge;

const WIDTH = 960;
const HEIGHT = 540;

const COLOR_BY_TYPE: Record<string, string> = {
  application: '#4ade80',
  route: '#60a5fa',
  background_worker: '#f97316',
  scheduler_job: '#a855f7',
  task: '#2dd4bf',
};

export const GraphCanvas = ({
  nodes,
  edges,
  onNodeSelect,
  selectedNode,
}: GraphCanvasProps) => {
  const [layoutNodes, setLayoutNodes] = useState<PositionedNode[]>([]);

  const links = useMemo<PositionedLink[]>(
    () =>
      edges.map((edge) => ({
        ...edge,
        source: edge.source,
        target: edge.target,
      })),
    [edges]
  );

  useEffect(() => {
    if (!nodes.length) {
      setLayoutNodes([]);
      return;
    }

    const simulationNodes: PositionedNode[] = nodes.map((node) => ({
      ...node,
      x: WIDTH / 2,
      y: HEIGHT / 2,
    }));

    const simulation = forceSimulation(simulationNodes)
      .force(
        'charge',
        forceManyBody()
          .strength(-260)
          .distanceMax(480)
      )
      .force('center', forceCenter(WIDTH / 2, HEIGHT / 2))
      .force(
        'link',
        forceLink<PositionedNode, PositionedLink>(links)
          .id((node: PositionedNode) => node.id)
          .distance(140)
          .strength(0.7)
      )
      .force('collision', forceCollide<PositionedNode>().radius(56))
      .on('tick', () => {
        simulationNodes.forEach((node) => {
          node.x = Math.max(60, Math.min(WIDTH - 60, node.x ?? WIDTH / 2));
          node.y = Math.max(60, Math.min(HEIGHT - 60, node.y ?? HEIGHT / 2));
        });
        setLayoutNodes(simulationNodes.map((node) => ({ ...node })));
      });

    const stopTimer = window.setTimeout(() => {
      simulation.stop();
    }, 2000);

    return () => {
      window.clearTimeout(stopTimer);
      simulation.stop();
    };
  }, [links, nodes]);

  const handleNodeClick = (node: GraphNode) => {
    onNodeSelect(node);
  };

  return (
    <div className="graph-container w-full overflow-hidden">
      <svg
        role="img"
        aria-label="Application graph"
        className="w-full h-[540px]"
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      >
        <g>
          {links.map((link) => {
            const resolveNode = (
              value: PositionedLink['source'] | PositionedLink['target']
            ) => {
              if (!value) {
                return undefined;
              }
              if (typeof value === 'string') {
                return layoutNodes.find((node) => node.id === value);
              }
              if (typeof value === 'number') {
                return layoutNodes[value];
              }
              if (typeof value === 'object' && 'id' in value) {
                const reference = value as PositionedNode;
                return layoutNodes.find((node) => node.id === reference.id);
              }
              return undefined;
            };

            const source = resolveNode(link.source);
            const target = resolveNode(link.target);
            if (!source || !target) {
              return null;
            }

            return (
              <line
                key={link.id}
                x1={source.x}
                y1={source.y}
                x2={target.x}
                y2={target.y}
                stroke="#334155"
                strokeWidth={2}
                strokeLinecap="round"
              />
            );
          })}
        </g>

        <g>
          {layoutNodes.map((node) => {
            const fill = COLOR_BY_TYPE[node.type] ?? '#94a3b8';
            const isSelected = selectedNode?.id === node.id;

            return (
              <g
                key={node.id}
                role="button"
                aria-label={`${node.type} node ${node.label}`}
                data-node-id={node.id}
                className={classNames('graph-node', {
                  'opacity-100': isSelected,
                  'opacity-80': !isSelected,
                })}
                onClick={() => handleNodeClick(node)}
              >
                <circle
                  cx={node.x}
                  cy={node.y}
                  r={isSelected ? 34 : 28}
                  fill={fill}
                  stroke={isSelected ? '#facc15' : '#0f172a'}
                  strokeWidth={isSelected ? 6 : 3}
                />
                <text
                  x={node.x}
                  y={node.y + (isSelected ? 48 : 42)}
                  textAnchor="middle"
                  className="fill-slate-100 text-sm"
                >
                  {node.label}
                </text>
              </g>
            );
          })}
        </g>
      </svg>
    </div>
  );
};
