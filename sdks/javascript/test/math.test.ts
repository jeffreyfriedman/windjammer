/**
 * Tests for math module.
 */

import { Vec2, Vec3, Vec4, Quat } from '../src/math';

describe('Vec2', () => {
  test('creation', () => {
    const v = new Vec2(1.0, 2.0);
    expect(v.x).toBe(1.0);
    expect(v.y).toBe(2.0);
  });

  test('addition', () => {
    const v1 = new Vec2(1.0, 2.0);
    const v2 = new Vec2(3.0, 4.0);
    const v3 = v1.add(v2);
    expect(v3.x).toBe(4.0);
    expect(v3.y).toBe(6.0);
  });

  test('subtraction', () => {
    const v1 = new Vec2(5.0, 6.0);
    const v2 = new Vec2(2.0, 3.0);
    const v3 = v1.sub(v2);
    expect(v3.x).toBe(3.0);
    expect(v3.y).toBe(3.0);
  });

  test('multiplication', () => {
    const v1 = new Vec2(2.0, 3.0);
    const v2 = v1.mul(2.0);
    expect(v2.x).toBe(4.0);
    expect(v2.y).toBe(6.0);
  });

  test('length', () => {
    const v = new Vec2(3.0, 4.0);
    expect(v.length()).toBe(5.0);
  });

  test('normalize', () => {
    const v = new Vec2(3.0, 4.0);
    const n = v.normalize();
    expect(Math.abs(n.length() - 1.0)).toBeLessThan(0.001);
  });

  test('dot', () => {
    const v1 = new Vec2(1.0, 2.0);
    const v2 = new Vec2(3.0, 4.0);
    expect(v1.dot(v2)).toBe(11.0);
  });

  test('zero', () => {
    const v = Vec2.zero();
    expect(v.x).toBe(0.0);
    expect(v.y).toBe(0.0);
  });

  test('one', () => {
    const v = Vec2.one();
    expect(v.x).toBe(1.0);
    expect(v.y).toBe(1.0);
  });
});

describe('Vec3', () => {
  test('creation', () => {
    const v = new Vec3(1.0, 2.0, 3.0);
    expect(v.x).toBe(1.0);
    expect(v.y).toBe(2.0);
    expect(v.z).toBe(3.0);
  });

  test('addition', () => {
    const v1 = new Vec3(1.0, 2.0, 3.0);
    const v2 = new Vec3(4.0, 5.0, 6.0);
    const v3 = v1.add(v2);
    expect(v3.x).toBe(5.0);
    expect(v3.y).toBe(7.0);
    expect(v3.z).toBe(9.0);
  });

  test('cross', () => {
    const v1 = new Vec3(1.0, 0.0, 0.0);
    const v2 = new Vec3(0.0, 1.0, 0.0);
    const v3 = v1.cross(v2);
    expect(v3.x).toBe(0.0);
    expect(v3.y).toBe(0.0);
    expect(v3.z).toBe(1.0);
  });

  test('up', () => {
    const v = Vec3.up();
    expect(v.x).toBe(0.0);
    expect(v.y).toBe(1.0);
    expect(v.z).toBe(0.0);
  });

  test('forward', () => {
    const v = Vec3.forward();
    expect(v.x).toBe(0.0);
    expect(v.y).toBe(0.0);
    expect(v.z).toBe(-1.0);
  });

  test('right', () => {
    const v = Vec3.right();
    expect(v.x).toBe(1.0);
    expect(v.y).toBe(0.0);
    expect(v.z).toBe(0.0);
  });
});

describe('Quat', () => {
  test('identity', () => {
    const q = Quat.identity();
    expect(q.x).toBe(0.0);
    expect(q.y).toBe(0.0);
    expect(q.z).toBe(0.0);
    expect(q.w).toBe(1.0);
  });
});

