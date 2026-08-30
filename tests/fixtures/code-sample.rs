//! Sample Rust file for the code → diagram generator.
//!
//! Try:
//!   diagram generate-class examples/code-sample.rs --output sample-class.mmd
//!   diagram generate-tree  examples/code-sample.rs --output sample-tree.mmd
//!   diagram generate-call  examples/code-sample.rs --output sample-call.mmd

use std::collections::HashMap;

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub enum Color {
    Red,
    Green,
    Blue,
}

pub trait Shape {
    fn area(&self) -> f64;
}

impl Shape for Point {
    fn area(&self) -> f64 {
        self.x * self.y
    }
}

pub fn compute(p: Point) -> f64 {
    let a = p.area();
    let next = adjust(a, 1.0);
    finalise(next)
}

fn adjust(v: f64, by: f64) -> f64 {
    v + by
}

fn finalise(v: f64) -> f64 {
    v
}