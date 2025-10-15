// In explorer-ui/src/types.ts

export interface GraphNode {
  id: string;
  label: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label: string;
}

export interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}