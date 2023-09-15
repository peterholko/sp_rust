use bevy::prelude::*;

#[derive(Clone, Component, Debug)]
pub struct ChaseAttack;

#[derive(Clone, Component, Debug)]
pub struct Chase;

#[derive(Clone, Component, Debug)]
pub struct VisibleTargetScorerBuilder;

#[derive(Component, Debug)]
pub struct VisibleTarget {
    pub target: i32,
}

impl VisibleTarget {
    pub fn new(target: i32) -> Self {
        Self { target }
    }
}