use odmo_application::game::RandomBoxRepository;
use odmo_types::RandomBoxReward;

use super::PgRepository;

impl RandomBoxRepository for PgRepository {
    /// Weighted reward pool a random box rolls a single reward from.
    ///
    /// The pool is built in code as a small demo set so the dev environment has
    /// rollable rewards without a dedicated catalog table.
    fn random_box_rewards(&self) -> anyhow::Result<Vec<RandomBoxReward>> {
        Ok(vec![
            RandomBoxReward {
                item_id: 5101,
                amount: 1,
                weight: 60,
            },
            RandomBoxReward {
                item_id: 5102,
                amount: 2,
                weight: 30,
            },
            RandomBoxReward {
                item_id: 90600,
                amount: 1,
                weight: 10,
            },
        ])
    }
}
