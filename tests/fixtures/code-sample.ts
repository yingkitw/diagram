// Sample TypeScript file for the code → diagram generator.
// Try:
//   diagram generate-class tests/fixtures/code-sample.ts --output class.mmd
//   diagram generate-tree  tests/fixtures/code-sample.ts --output tree.mmd
//   diagram generate-call  tests/fixtures/code-sample.ts --output call.mmd

export interface Shape {
    area(): number;
}

export class Circle implements Shape {
    constructor(public radius: number) {}
    area(): number {
        return Math.PI * this.radius * this.radius;
    }
}

export class Point {
    public x: number;
    public y: number;
}

export enum Color {
    Red,
    Green,
    Blue,
}

export function compute(p: Point): number {
    const a = p.area();
    return adjust(a, 1.0);
}

function adjust(v: number, by: number): number {
    return v + by;
}

import { HashMap } from "std-collections";