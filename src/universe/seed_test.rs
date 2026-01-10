#[cfg(test)]
mod tests {
    use crate::universe::get_universe_seed;
    use std::env;
    use std::sync::Mutex;

    // We use a static mutex to ensure that tests modifying environment variables
    // do not run concurrently, preventing race conditions.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_get_universe_seed_random() {
        let _lock = ENV_LOCK.lock().unwrap();

        // Without env var, it should be random.
        unsafe { env::remove_var("UNIVERSE_SEED"); }
        let seed1 = get_universe_seed();
        let seed2 = get_universe_seed();
        // There's a tiny chance they match, but it's 1 in 2^64.
        assert_ne!(seed1, seed2, "Random seeds should likely differ");
    }

    #[test]
    fn test_get_universe_seed_env_var() {
        let _lock = ENV_LOCK.lock().unwrap();

        unsafe { env::set_var("UNIVERSE_SEED", "99999"); }
        let seed = get_universe_seed();
        assert_eq!(seed, 99999);
        unsafe { env::remove_var("UNIVERSE_SEED"); }
    }

    #[test]
    fn test_get_universe_seed_invalid_env_var() {
        let _lock = ENV_LOCK.lock().unwrap();

        unsafe { env::set_var("UNIVERSE_SEED", "invalid"); }
        let seed = get_universe_seed();
        // Should fall back to random
        // We can't assert value but we can assert it doesn't panic.
        println!("Seed from invalid: {}", seed);
        unsafe { env::remove_var("UNIVERSE_SEED"); }
    }
}
