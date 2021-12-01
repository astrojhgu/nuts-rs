use crate::{
    cpu_potential::{Potential, UnitMassMatrix},
    cpu_state::{State, StatePool},
    nuts::{draw, Collector, SampleInfo},
};

pub use crate::cpu_potential::CpuLogpFunc;


struct RunningMean {
    sum: f64,
    count: u64,
}

impl RunningMean {
    fn new() -> RunningMean {
        RunningMean {
            sum: 0.,
            count: 0,
        }
    }

    fn add(&mut self, value: f64) {
        self.sum += value;
        self.count += 1;
    }

    fn current(&self) -> f64 {
        self.sum / self.count as f64
    }

    fn reset(&mut self) {
        self.sum = 0f64;
        self.count = 0;
    }
}

struct AcceptanceRateCollector {
    initial_energy: f64,
    mean: RunningMean,
}


impl AcceptanceRateCollector {
    fn new() -> AcceptanceRateCollector {
        AcceptanceRateCollector {
            initial_energy: 0.,
            mean: RunningMean::new(),
        }
    }
}

impl Collector for AcceptanceRateCollector {
    type State = State;

    fn register_leapfrog(
        &mut self,
        _start: &Self::State,
        end: &Self::State,
        _step_size: f64,
        _divergence_info: Option<&dyn crate::nuts::DivergenceInfo>,
    ) {
        use crate::nuts::State;
        self.mean.add(end.log_acceptance_probability(self.initial_energy).exp())
    }

    fn register_init(&mut self, state: &Self::State) {
        use crate::nuts::State;
        self.initial_energy = state.energy();
        self.mean.reset();
    }
}


struct StatsCollector {
    acceptance_rate: AcceptanceRateCollector,
}


#[derive(Debug)]
pub struct Stats {
    pub mean_acceptance_rate: f64,
}


impl StatsCollector {
    fn new() -> StatsCollector {
        StatsCollector {
            acceptance_rate: AcceptanceRateCollector::new(),
        }
    }

    fn stats(&self) -> Stats {
        Stats {
            mean_acceptance_rate: self.acceptance_rate.mean.current(),
        }
    }
}


impl Collector for StatsCollector {
    type State = State;

    fn register_leapfrog(
        &mut self,
        start: &Self::State,
        end: &Self::State,
        step_size: f64,
        divergence_info: Option<&dyn crate::nuts::DivergenceInfo>,
    ) {
        self.acceptance_rate.register_leapfrog(start, end, step_size, divergence_info);
    }

    fn register_draw(&mut self, state: &Self::State, info: &SampleInfo) {
        self.acceptance_rate.register_draw(state, info);
    }

    fn register_init(&mut self, state: &Self::State) {
        self.acceptance_rate.register_init(state);
    }
}


pub struct UnitStaticSampler<F: CpuLogpFunc> {
    potential: Potential<F, UnitMassMatrix>,
    state: State,
    pool: StatePool,
    maxdepth: u64,
    step_size: f64,
    rng: rand::rngs::StdRng,
    collector: StatsCollector,
}

struct NullCollector {}

impl Collector for NullCollector {
    type State = State;
}

impl<F: CpuLogpFunc> UnitStaticSampler<F> {
    pub fn new(logp: F, seed: u64, maxdepth: u64, step_size: f64) -> UnitStaticSampler<F> {
        use rand::SeedableRng;

        let mass_matrix = UnitMassMatrix {};
        let mut pool = StatePool::new(logp.dim());
        let potential = Potential::new(logp, mass_matrix);
        let state = pool.new_state();
        let collector = StatsCollector::new();
        UnitStaticSampler {
            potential,
            state,
            pool,
            maxdepth,
            step_size,
            rng: rand::rngs::StdRng::seed_from_u64(seed),
            collector,
        }
    }

    pub fn set_position(&mut self, position: &[f64]) -> Result<(), F::Err> {
        use crate::nuts::Potential;
        {
            let inner = self.state.try_mut_inner().expect("State already in use");
            inner.q.copy_from_slice(position);
        }
        if let Err(err) = self.potential.update_potential_gradient(&mut self.state) {
            return Err(err.logp_function_error.unwrap());
        }
        // TODO check init of p_sum
        Ok(())
    }

    pub fn draw(&mut self) -> (Box<[f64]>, SampleInfo, Stats) {
        use crate::nuts::Potential;
        self.potential.randomize_momentum(&mut self.state, &mut self.rng);
        self.potential.update_velocity(&mut self.state);
        self.potential.update_kinetic_energy(&mut self.state);

        let (state, info) = draw(
            &mut self.pool,
            self.state.clone(),
            &mut self.rng,
            &mut self.potential,
            self.maxdepth,
            self.step_size,
            &mut self.collector,
        );
        self.state = state;
        let position: Box<[f64]> = self.state.q.clone().into();
        (position, info, self.collector.stats())
    }
}

pub mod test_logps {
    use crate::cpu_potential::CpuLogpFunc;

    pub struct NormalLogp {
        dim: usize,
        mu: f64,
    }

    impl NormalLogp {
        pub fn new(dim: usize, mu: f64) -> NormalLogp {
            NormalLogp { dim, mu }
        }
    }

    impl CpuLogpFunc for NormalLogp {
        type Err = ();

        fn dim(&self) -> usize {
            self.dim
        }
        fn logp(&mut self, position: &[f64], gradient: &mut [f64]) -> Result<f64, ()> {
            let n = position.len();
            assert!(gradient.len() == n);

            let mut logp = 0f64;
            for (p, g) in position.iter().zip(gradient.iter_mut()) {
                let val = *p - self.mu;
                logp -= val * val;
                *g = -val;
            }
            Ok(logp)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cpu_sampler::UnitStaticSampler;

    use super::test_logps::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn make_state() {
        /*
        let _ = State::new(10);
        let mut storage = Rc::new(StateStorage::with_capacity(0));
        let a = new_state(&mut storage, 10);
        assert!(storage.free_states.borrow_mut().len() == 0);
        drop(a);
        assert!(storage.free_states.borrow_mut().len() == 1);
        let a = new_state(&mut storage, 10);
        assert!(storage.free_states.borrow_mut().len() == 0);
        drop(a);
        assert!(storage.free_states.borrow_mut().len() == 1);
        */
    }

    #[test]
    fn deterministic() {
        let dim = 3usize;
        let func = NormalLogp::new(dim, 3.);
        let init = vec![3.5; dim];

        let mut sampler = UnitStaticSampler::new(func, 42, 10, 1e-2);

        sampler.set_position(&init).unwrap();
        let (sample1, info1, _stats) = sampler.draw();

        let func = NormalLogp::new(dim, 3.);
        let mut sampler = UnitStaticSampler::new(func, 42, 10, 1e-2);

        sampler.set_position(&init).unwrap();
        let (sample2, info2, _stats) = sampler.draw();

        dbg!(&sample1);
        dbg!(info1);

        dbg!(&sample2);
        dbg!(info2);

        assert_eq!(sample1, sample2);
    }
}
