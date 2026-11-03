//! The deterministic random number generator.

use isocity_core::Rng;

#[test]
fn deterministic() {
    let mut a = Rng::new(42);
    let mut b = Rng::new(42);
    for _ in 0..100 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn floats_in_range() {
    let mut r = Rng::new(7);
    for _ in 0..10_000 {
        let f = r.next_f64();
        assert!((0.0..1.0).contains(&f));
        assert!(r.below(5) < 5);
    }
}
