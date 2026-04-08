# clock-rs

A minimal, testable `Clock` abstraction for Rust, backed by `chrono`.

## Motivation

Production code that calls `Utc::now()` directly is hard to test deterministically. Time-dependent
logic — expiry checks, scheduling, audit timestamps — becomes non-reproducible and flaky when the
clock is a global singleton.

`clock-rs` solves this with a single trait and two implementations:

- **`SystemClock`** delegates to the real wall clock and is used in production.
- **`FakeClock`** (feature-gated) holds a controllable, in-memory timestamp for use in tests.

Inject `Arc<dyn Clock>` into your types. In production wire up `SystemClock`; in tests hand in a
`FakeClock` and advance it however you need. No mocking framework required, no `#[cfg(test)]`
scattered through your business logic.

## Features

- Zero-overhead `Clock` trait (`Send + Sync`) — one method, `now() -> DateTime<Utc>`
- `SystemClock` — thin wrapper around `Utc::now()`
- `FakeClock` — deterministic fake with `set_now` and `advance`, protected by a `parking_lot`
  read-write lock so it is safe to share across threads
- `FakeClock` is compiled only when the `test-utils` feature is enabled — zero production overhead
- Minimal dependencies: only `chrono` in the default build; `parking_lot` is opt-in

## Installation

```toml
[dependencies]
clock-rs = "0.1"

[dev-dependencies]
clock-rs = { version = "0.1", features = ["test-utils"] }
```

Enable `test-utils` only under `[dev-dependencies]` so `FakeClock` and its `parking_lot` dependency
never appear in your production binary.

## Usage

### Production code

Accept `Arc<dyn Clock>` so the dependency can be swapped at the call site:

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

### Controlling time in tests

Use `FakeClock` to pin the clock to a known instant and advance it programmatically:

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
        clock.advance(chrono::Duration::hours(2)); // jump past expiry
        let valid = validator.is_valid();

        // assert
        assert!(!valid);
    }
}
```

`FakeClock::advance` is safe to call concurrently — the inner timestamp is guarded by a
`parking_lot::RwLock`.

### Setting an absolute time

```rust
use clock_rs::test_utils::FakeClock;
use chrono::{TimeZone, Utc};

let clock = FakeClock::new(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
clock.set_now(Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap());
assert_eq!(clock.now().year(), 2025);
```

## Design notes

**Why a trait instead of a type alias or generic parameter?**
A trait object (`Arc<dyn Clock>`) keeps the concrete implementation out of struct signatures and
avoids monomorphising every type that touches the clock. It also makes the injection point
explicit — a clear seam for testing.

**Why feature-gate `FakeClock`?**
`FakeClock` pulls in `parking_lot`. Gating it behind `test-utils` keeps the default dependency
graph lean and signals that this type is not for production use. Crates that want it opt in
deliberately, typically only under `[dev-dependencies]`.

**Why `parking_lot::RwLock` instead of `std::sync::RwLock`?**
`parking_lot`'s implementation is faster on uncontended paths, does not poison on panic, and has a
smaller memory footprint — all desirable properties for a clock used in a test harness that may
spawn many threads.

## License

MIT
