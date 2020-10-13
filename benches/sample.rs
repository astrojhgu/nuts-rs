use criterion::{black_box, criterion_group, criterion_main, Criterion};


pub fn sample_one(mu: f64, out: &mut [f64]) {
    use nuts_rs::nuts::Integrator;

    struct NormalLogp { dim: usize, mu: f64 };

    impl nuts_rs::cpu::LogpFunc for NormalLogp {
        fn dim(&self) -> usize {
            self.dim
        }
        fn logp(&self, state: &mut nuts_rs::cpu::InnerState) -> f64 {
            let position = &state.q;
            let grad = &mut state.grad;
            let n = position.len();
            assert!(grad.len() == n);
            let mut logp = 0f64;
            for i in 0..n {
                let val = position[i] - self.mu;
                logp -= val * val;
                grad[i] = -val;
            }
            logp
        }
    }

    let func = NormalLogp { dim: 10, mu };
    let init = vec![3.5; func.dim];
    let mut integrator = nuts_rs::cpu::StaticIntegrator::new(func, &init);

    let mut rng = rand::thread_rng();
    
    integrator.randomize_initial(&mut rng);
    let (state, info) = nuts_rs::nuts::draw(&mut rng, &mut integrator, 20);
    integrator.write_position(&state, out);
}


fn criterion_benchmark(c: &mut Criterion) {
    let mut out = vec![0.; 10];
    c.bench_function("sample normal 10", |b| b.iter(|| sample_one(black_box(3.), black_box(&mut out))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);