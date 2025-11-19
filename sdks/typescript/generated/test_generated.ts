/**
 * Test generated TypeScript SDK
 */
import { App } from './app';
import { Vec2 } from './vec2';
import { Vec3 } from './vec3';
import { World } from './world';
import { Entity } from './entity';
import { Key } from './key';
import { MouseButton } from './mousebutton';

function testImports(): void {
    console.log('✓ All imports successful');
}

function testVec2(): void {
    const v: Vec2 = { x: 1.0, y: 2.0 };
    if (v.x !== 1.0 || v.y !== 2.0) {
        throw new Error('Vec2 test failed');
    }
    console.log('✓ Vec2 works');
}

function testVec3(): void {
    const v: Vec3 = { x: 1.0, y: 2.0, z: 3.0 };
    if (v.x !== 1.0 || v.y !== 2.0 || v.z !== 3.0) {
        throw new Error('Vec3 test failed');
    }
    console.log('✓ Vec3 works');
}

function testEnums(): void {
    if (Key.A === undefined) {
        throw new Error('Key enum test failed');
    }
    if (MouseButton.Left === undefined) {
        throw new Error('MouseButton enum test failed');
    }
    console.log('✓ Enums work');
}

function testClasses(): void {
    const app = new App();
    const world = new World();
    const entity = new Entity();
    console.log('✓ Classes instantiate');
}

function runTests(): void {
    testImports();
    testVec2();
    testVec3();
    testEnums();
    testClasses();
    console.log('\n🎉 All generated SDK tests passed!');
}

runTests();

