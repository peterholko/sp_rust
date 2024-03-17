use bevy::prelude::*;
use big_brain::prelude::*;

use crate::game::Position;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct ChaseAndAttack;

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

// Necromancer
#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct ChaseAndCast {
    pub start_time: i32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct RaiseDead {
    pub start_time: i32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct FleeToHome;

// Corpse targets for Necromancer
#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct VisibleCorpseScorer;

// Corpse targets for Necromancer
#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct FleeScorer;

#[derive(Debug, Component)]
pub struct VisibleCorpse {
    pub corpse: i32,
}

impl VisibleCorpse {
    pub fn new(corpse: i32) -> Self {
        Self { corpse }
    }
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Hide;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct MerchantScorer;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct SailToPort;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Transport {
    pub route: Vec<Position>,
    pub next_stop: i32,
    pub hauling: Vec<i32>,
}

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct TaxCollector {
    pub target_player: i32,
    pub collection_amount: i32,
    pub debt_amount: i32,
    pub last_collection_time: i32,
    pub landing_pos: Position,
    pub transport_id: i32,
    pub last_demand_time: i32,
}

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsAboard;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsTaxCollected;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct AtLanding;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct OverdueTaxScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct NoTaxesToCollect;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TaxesToCollect;


#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct SetDestination;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Idle {
    pub start_time: i32,
    pub duration: i32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Talk {
    pub speech: String,
}


#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Destination {
    pub pos: Position
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToTarget {
    pub target: i32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToPos;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToEmpire;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Forfeiture;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct TaxCollectorTransport {
    pub tax_collector_id: i32
}