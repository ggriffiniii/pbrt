use std::ops::{Add, Div, Mul};

use core::pbrt::{Float, Int};

pub trait Sqrt<RHS = Self> {
    type Output;
    fn sqrt(self) -> Self::Output;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vector2<T> {
    pub x: T,
    pub y: T,
}

pub type Vector2f = Vector2<Float>;
pub type Vector2i = Vector2<Int>;

#[derive(Debug, Clone, PartialEq)]
pub struct Vector3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

impl<T> Vector3<T> {
    pub fn new(x: T, y: T, z: T) -> Vector3<T> {
        Vector3 { x, y, z }
    }
}

pub type Vector3f = Vector3<Float>;

// TODO(wathiede): Make this generic over float vs int.
impl Vector3f {
    pub fn normalize(&self) -> Vector3f {
        self / self.length()
    }

    pub fn length_squared(&self) -> Float {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    pub fn length(&self) -> Float {
        self.length_squared().sqrt()
    }
}

// TODO(wathiede): Make this generic over float vs int.
impl<'a> Div<Float> for &'a Vector3f {
    type Output = Vector3f;

    fn div(self, rhs: Float) -> Vector3f {
        Vector3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

pub type Vector3i = Vector3<Int>;

#[derive(Debug, Clone, PartialEq)]
pub struct Point2<T> {
    pub x: T,
    pub y: T,
}

pub type Point2f = Point2<Float>;
pub type Point2i = Point2<Int>;

#[derive(Debug, Clone, PartialEq)]
pub struct Point3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Point3f = Point3<Float>;
pub type Point3i = Point3<Int>;

#[derive(Debug, Clone, PartialEq)]
pub struct Normal3<T> {
    pub x: T,
    pub y: T,
    pub z: T,
}

pub type Normal3f = Normal3<Float>;