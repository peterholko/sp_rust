use bevy::prelude::*;
use big_brain::prelude::*;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct ChaseAttack;

#[derive(Debug, Clone, Component)]
pub struct Chase;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct VisibleTargetScorer;

#[derive(Debug, Component)]
pub struct VisibleTarget {
    pub target: i32,
}

impl VisibleTarget {
    pub fn new(target: i32) -> Self {
        Self { target }
    }
}