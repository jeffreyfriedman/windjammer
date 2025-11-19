/**
 * AI components.
 */

import { Vec3 } from './math';

/**
 * Behavior tree for AI decision making.
 */
export class BehaviorTree {
  private root: any = null;

  /**
   * Execute one tick of the behavior tree.
   * 
   * @returns The status of the tick
   */
  tick(): string {
    return 'success';
  }

  toString(): string {
    return 'BehaviorTree()';
  }
}

/**
 * Pathfinding component.
 */
export class Pathfinder {
  /** Current path */
  path: Vec3[] = [];

  /**
   * Find a path from start to goal.
   * 
   * @param start - Start position
   * @param goal - Goal position
   * @returns The calculated path
   */
  findPath(start: Vec3, goal: Vec3): Vec3[] {
    console.log(`[Pathfinding] Finding path from ${start} to ${goal}`);
    return [start, goal];
  }

  toString(): string {
    return `Pathfinder(pathLength=${this.path.length})`;
  }
}

