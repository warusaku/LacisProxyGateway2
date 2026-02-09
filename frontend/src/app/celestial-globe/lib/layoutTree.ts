// CelestialGlobe v2 — Layout Tree
// mobes2.0 lib/layoutSimple.ts (705行) 準拠
// DFS深さ優先でsubtreeの高さを再帰計算し、各ノードを配置する。

import type { Node } from 'reactflow';
import type { TopologyNodeV2 } from '../types';
import { LAYOUT } from '../constants';

// ============================================================================
// Types
// ============================================================================

export interface LayoutOptions {
  direction?: 'LR' | 'RL';
  siblingGap?: number;
  depthSpacing?: number;
  nodeHeight?: (nodeType: string) => number;
}

export interface LayoutResult {
  nodes: Node[];
  depthMap: Map<string, number>;
}

// ============================================================================
// Tree builder
// ============================================================================

interface TreeNode {
  id: string;
  node: TopologyNodeV2;
  children: TreeNode[];
}

function buildTree(
  nodes: TopologyNodeV2[],
): { root: TreeNode | null; orphans: TreeNode[] } {
  const nodeMap = new Map<string, TreeNode>();

  // Create TreeNode for each topology node
  for (const n of nodes) {
    nodeMap.set(n.id, { id: n.id, node: n, children: [] });
  }

  let root: TreeNode | null = null;
  const orphans: TreeNode[] = [];

  for (const n of nodes) {
    const treeNode = nodeMap.get(n.id)!;

    if (n.node_type === 'internet') {
      root = treeNode;
      continue;
    }

    if (n.parent_id) {
      const parent = nodeMap.get(n.parent_id);
      if (parent) {
        parent.children.push(treeNode);
      } else {
        orphans.push(treeNode);
      }
    } else {
      orphans.push(treeNode);
    }
  }

  // Sort children by order
  nodeMap.forEach((tn) => {
    tn.children.sort((a: TreeNode, b: TreeNode) => a.node.order - b.node.order);
  });

  return { root, orphans };
}

// ============================================================================
// Layout computation
// ============================================================================

function getNodeHeight(nodeType: string, customFn?: (t: string) => number): number {
  if (customFn) return customFn(nodeType);
  return LAYOUT.NODE_HEIGHT_DEFAULT;
}

function computeSubtreeHeight(
  treeNode: TreeNode,
  gap: number,
  heightFn?: (t: string) => number,
): number {
  if (treeNode.children.length === 0) {
    return getNodeHeight(treeNode.node.node_type, heightFn);
  }

  let totalChildHeight = 0;
  for (let i = 0; i < treeNode.children.length; i++) {
    totalChildHeight += computeSubtreeHeight(treeNode.children[i], gap, heightFn);
    if (i < treeNode.children.length - 1) {
      totalChildHeight += gap;
    }
  }

  const selfHeight = getNodeHeight(treeNode.node.node_type, heightFn);
  return Math.max(selfHeight, totalChildHeight);
}

function positionSubtree(
  treeNode: TreeNode,
  x: number,
  yStart: number,
  depth: number,
  gap: number,
  depthSpacing: number,
  direction: 'LR' | 'RL',
  heightFn: ((t: string) => number) | undefined,
  result: Node[],
  depthMap: Map<string, number>,
): void {
  const subtreeH = computeSubtreeHeight(treeNode, gap, heightFn);
  const selfH = getNodeHeight(treeNode.node.node_type, heightFn);
  const centerY = yStart + subtreeH / 2 - selfH / 2;

  const posX = direction === 'LR' ? x : -x;

  result.push({
    id: treeNode.id,
    type: treeNode.node.node_type === 'internet' ? 'internet' : 'device',
    position: { x: posX, y: centerY },
    data: { node: treeNode.node },
  });
  depthMap.set(treeNode.id, depth);

  // Position children
  let childY = yStart;
  for (const child of treeNode.children) {
    const childSubtreeH = computeSubtreeHeight(child, gap, heightFn);
    positionSubtree(
      child,
      x + depthSpacing,
      childY,
      depth + 1,
      gap,
      depthSpacing,
      direction,
      heightFn,
      result,
      depthMap,
    );
    childY += childSubtreeH + gap;
  }
}

// ============================================================================
// Public API
// ============================================================================

export function layoutTree(
  nodes: TopologyNodeV2[],
  options?: LayoutOptions,
): LayoutResult {
  const gap = options?.siblingGap ?? LAYOUT.SIBLING_GAP;
  const depthSpacing = options?.depthSpacing ?? LAYOUT.DEPTH_SPACING;
  const direction = options?.direction ?? 'LR';
  const heightFn = options?.nodeHeight;

  const { root, orphans } = buildTree(nodes);

  const resultNodes: Node[] = [];
  const depthMap = new Map<string, number>();

  if (root) {
    // Attach orphans to root
    const rootTree = buildTree(nodes).root;
    if (rootTree) {
      for (const orphan of orphans) {
        rootTree.children.push(orphan);
      }
      rootTree.children.sort((a, b) => a.node.order - b.node.order);

      positionSubtree(
        rootTree,
        0,
        0,
        0,
        gap,
        depthSpacing,
        direction,
        heightFn,
        resultNodes,
        depthMap,
      );
    }
  } else if (orphans.length > 0) {
    // No internet node — lay out orphans linearly
    let y = 0;
    for (const orphan of orphans) {
      positionSubtree(
        orphan,
        0,
        y,
        0,
        gap,
        depthSpacing,
        direction,
        heightFn,
        resultNodes,
        depthMap,
      );
      y += computeSubtreeHeight(orphan, gap, heightFn) + gap;
    }
  }

  // Normalize: ensure no negative Y positions
  let minY = Infinity;
  for (const n of resultNodes) {
    if (n.position.y < minY) minY = n.position.y;
  }
  if (minY < 0) {
    for (const n of resultNodes) {
      n.position.y -= minY;
    }
  }

  return { nodes: resultNodes, depthMap };
}
