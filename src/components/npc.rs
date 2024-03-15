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
pub struct ChaseAndCast;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct RaiseDead;

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

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct MerchantScorer;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct SailToPort;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TaxCollectorShipScorer;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct Transport {
    pub route: Vec<Position>,
    pub next_stop: i32,
    pub hauling: Vec<i32>,
}

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct WaitForPassenger {
    pub id: i32,
}

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct PassengerOn {
    pub id: i32,
}

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TaxCollectorScorer;

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
pub struct InEmpire;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct AtLanding;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct AtDestinationScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsPassengerAboard;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsWaitingForPassenger;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct ReadyToSailScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsTargetAdjacent;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct OverdueTaxScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct NoTaxesToCollect;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TaxesToCollect;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Idle;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct SetDestination;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToTarget {
    pub target: i32,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToPos {
    pub pos: Position,
}

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct MoveToEmpire;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Forfeiture;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct TaxCollectorTransport {
    pub tax_collector_id: i32
}