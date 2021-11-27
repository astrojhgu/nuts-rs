use std::fmt::Debug;

use crate::cpu_state::{InnerState, State};
use crate::math::norm;
use crate::nuts::DivergenceInfo;

pub trait CpuLogpFunc {
    type Err: Debug + 'static;

    fn logp(&mut self, position: &[f64], grad: &mut [f64]) -> Result<f64, Self::Err>;
    fn dim(&self) -> usize;
}

pub(crate) trait MassMatrix {
    fn update_velocity(&self, state: &mut InnerState);
    fn update_kinetic_energy(&self, state: &mut InnerState);
    fn randomize_momentum<R: rand::Rng + ?Sized>(&self, state: &mut InnerState, rng: &mut R);
}

pub(crate) struct UnitMassMatrix {}

impl MassMatrix for UnitMassMatrix {
    fn update_velocity(&self, state: &mut InnerState) {
        state.v.copy_from_slice(&state.p);
    }

    fn update_kinetic_energy(&self, state: &mut InnerState) {
        state.kinetic_energy = 0.5 * norm(&state.v);
    }

    fn randomize_momentum<R: rand::Rng + ?Sized>(&self, state: &mut InnerState, rng: &mut R) {
        let dist = rand_distr::StandardNormal;
        for val in state.p.iter_mut() {
            *val = rng.sample(dist);
        }
    }
}

pub(crate) struct DiagMassMatrix {
    diag: Box<[f64]>,
}

impl MassMatrix for DiagMassMatrix {
    fn update_velocity(&self, state: &mut InnerState) {
        todo!()
    }

    fn update_kinetic_energy(&self, state: &mut InnerState) {
        todo!()
    }

    fn randomize_momentum<R: rand::Rng + ?Sized>(&self, state: &mut InnerState, rng: &mut R) {
        todo!()
    }
    /*
     */
}

#[derive(Debug)]
pub struct DivergenceInfoImpl<E> {
    pub logp_function_error: Option<E>,
}

impl<E: Debug> DivergenceInfo for DivergenceInfoImpl<E> {}

pub(crate) struct Potential<F: CpuLogpFunc, M: MassMatrix> {
    logp: F,
    diag: Box<[f64]>,
    inv_diag: Box<[f64]>,
    mass_matrix: M,
}

impl<F: CpuLogpFunc, M: MassMatrix> Potential<F, M> {
    pub(crate) fn new(logp: F, mass_matrix: M) -> Potential<F, M> {
        let dim = logp.dim();

        let potential = Potential {
            logp,
            diag: vec![1.; dim].into(),
            inv_diag: vec![1.; dim].into(),
            mass_matrix,
        };

        potential
    }

    pub(crate) fn mass_matrix(&self) -> &M {
        &self.mass_matrix
    }

    pub(crate) fn mass_matrix_mut(&mut self) -> &mut M {
        &mut self.mass_matrix
    }
}

impl<F: CpuLogpFunc, M: MassMatrix> crate::nuts::Potential for Potential<F, M> {
    type State = State;
    type DivergenceInfo = DivergenceInfoImpl<F::Err>;

    fn update_potential_gradient(&mut self, state: &mut State) -> Result<(), Self::DivergenceInfo> {
        let inner = state.try_mut_inner().unwrap();
        inner.potential_energy =
            -self
                .logp
                .logp(&inner.q[..], &mut inner.grad[..])
                .map_err(|err| DivergenceInfoImpl {
                    logp_function_error: Some(err),
                })?;
        Ok(())
    }

    fn randomize_momentum<R: rand::Rng + ?Sized>(&self, state: &mut Self::State, rng: &mut R) {
        let inner = state.try_mut_inner().unwrap();
        self.mass_matrix.randomize_momentum(inner, rng);
    }

    fn update_velocity(&mut self, state: &mut Self::State) {
        self.mass_matrix
            .update_velocity(state.try_mut_inner().expect("State already in us"))
    }

    fn update_kinetic_energy(&mut self, state: &mut Self::State) {
        self.mass_matrix
            .update_kinetic_energy(state.try_mut_inner().expect("State already in us"))
    }
}