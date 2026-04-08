# clock-rs

A minimal `Clock` abstraction for Rust, backed by `chrono`.

## Why

Calling `Utc::now()` directly makes time-dependent logic — expiry checks, scheduling, audit
timestamps — impossible to test deterministically. This crate gives you a `Clock` trait with two
implementations: `SystemClock` for production, and `FakeClock` for tests. Inject `Arc<dyn Clock>`,
swap the implementation at the boundary, done.

No mocking framework. No `#[cfg(test)]` leaking into business logic.

## Installation

```toml
[dependencies]
clock-rs = "0.1"

[dev-dependencies]
clock-rs = { version = "0.1", features = ["test-utils"] }
```

`FakeClock` lives behind the `test-utils` feature so it — and its `parking_lot` dependency — never
end up in your production binary.

## Usage

Accept `Arc<dyn Clock>` in your types:

```rust
use clock_rs::{Clock, SystemClock};
use chrono::{DateTime, Utc};
use std::sync::Arc;

struct TokenValidator {
    clock: Arc<dyn Clock>,
    expiry: DateTime<Utc>,
}

impl TokenValidator {
    pub fn new(clock: Arc<dyn Clock>, expiry: DateTime<Utc>) -> Self {
        Self { clock, expiry }
    }

    pub fn is_valid(&self) -> bool {
        self.clock.now() < self.expiry
    }
}

fn main() {
    let clock = Arc::new(SystemClock);
    let expiry = Utc::now() + chrono::Duration::hours(1);
    let validator = TokenValidator::new(clock, expiry);
    println!("token valid: {}", validator.is_valid());
}
```

In tests, hand in a `FakeClock` and advance it however you need:

```rust
#[cfg(test)]
mod tests {
    use clock_rs::test_utils::FakeClock;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    use super::TokenValidator;

    fn t(h: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2024, 1, 1, h, 0, 0).unwrap()
    }

    #[test]
    fn given_clock_before_expiry_then_token_is_valid() {
        // arrange
        let clock = Arc::new(FakeClock::new(t(0)));
        let validator = TokenValidator::new(clock.clone(), t(1));

        // act
        let valid = validator.is_valid();

        // assert
        assert!(valid);
    }

    #[test]
    fn given_clock_after_expiry_then_token_is_invalid() {
        // arrange
        let clock = Arc::new(FakeClock::new(t(0)));
        let validator = TokenValidator::new(clock.clone(), t(1));

        // act
        clock.advance(chrono::Duration::hours(2));
        let valid = validator.is_valid();

        // assert
        assert!(!valid);
    }
}
```

`FakeClock::advance` and `FakeClock::set_now` are both safe to call concurrently — the inner
timestamp is guarded by a `parking_lot::RwLock`.

## Design notes

**Why a trait object instead of a generic?**
`Arc<dyn Clock>` keeps the concrete type out of struct signatures and avoids monomorphising every
type that touches a clock. The injection point stays explicit and the seam for testing is obvious.

**Why feature-gate `FakeClock`?**
It pulls in `parking_lot`. Gating it behind `test-utils` keeps the default dependency graph lean
and makes it clear this type isn't for production. Crates opt in deliberately, usually only under
`[dev-dependencies]`.

**Why `parking_lot::RwLock` over `std::sync::RwLock`?**
Faster on uncontended paths, no poisoning on panic, smaller footprint. All reasonable properties
for a lock that mostly exists to make test infrastructure thread-safe.

## License

MIT
