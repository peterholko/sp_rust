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

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct TransportTaxCollector;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct TaxCollectorShip {
    pub home_port: Position,
    pub target_port: Position,
    pub dest: Position,
    pub in_port_at: i32,
    pub hauling: Vec<i32>,
}

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct TaxCollectorScorer;

#[derive(Debug, Reflect, Component, Default)]
#[reflect(Component)]
pub struct TaxCollector {
    pub target_player: i32,
    pub collection_amount: i32,
    pub landing_pos: Position,
    pub transport_id: i32
}

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsAboardScorer;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsTaxCollected;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct InEmpire;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct AtLanding;

#[derive(Debug, Clone, Component, ScorerBuilder)]
pub struct IsHeroNearby;


#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Idle;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Embark;

#[derive(Debug, Clone, Component, ActionBuilder)]
pub struct Disembark;

