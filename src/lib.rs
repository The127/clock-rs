use chrono::{DateTime, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[cfg(feature = "test-utils")]
pub mod test_utils {
    use super::*;
    use std::sync::Arc;

    pub struct FakeClock {
        now: Arc<parking_lot::RwLock<DateTime<Utc>>>,
    }

    impl FakeClock {
        pub fn new(now: DateTime<Utc>) -> Self {
            Self {
                now: Arc::new(parking_lot::RwLock::new(now)),
            }
        }

        pub fn set_now(&self, now: DateTime<Utc>) {
            *self.now.write() = now;
        }

        pub fn advance(&self, duration: chrono::Duration) {
            let mut guard = self.now.write();
            *guard = *guard + duration;
        }
    }

    impl Clock for FakeClock {
        fn now(&self) -> DateTime<Utc> {
            *self.now.read()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_system_clock_then_now_returns_current_time() {
        // arrange
        let clock = SystemClock;

        // act
        let before = Utc::now();
        let t = clock.now();
        let after = Utc::now();

        // assert
        assert!(t >= before);
        assert!(t <= after);
    }

    #[cfg(feature = "test-utils")]
    mod fake_clock_tests {
        use super::*;
        use crate::test_utils::FakeClock;
        use chrono::TimeZone;

        fn fixed_time() -> DateTime<Utc> {
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
        }

        #[test]
        fn given_fake_clock_then_now_returns_set_time() {
            // arrange
            let clock = FakeClock::new(fixed_time());

            // act
            let t = clock.now();

            // assert
            assert_eq!(t, fixed_time());
        }

        #[test]
        fn given_fake_clock_then_set_now_updates_time() {
            // arrange
            let clock = FakeClock::new(fixed_time());
            let new_time = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();

            // act
            clock.set_now(new_time);

            // assert
            assert_eq!(clock.now(), new_time);
        }

        #[test]
        fn given_fake_clock_then_advance_moves_time_forward() {
            // arrange
            let clock = FakeClock::new(fixed_time());
            let duration = chrono::Duration::hours(3);

            // act
            clock.advance(duration);

            // assert
            let expected = Utc.with_ymd_and_hms(2024, 1, 1, 3, 0, 0).unwrap();
            assert_eq!(clock.now(), expected);
        }

        #[test]
        fn given_fake_clock_implements_clock_trait_then_usable_as_dyn_clock() {
            // arrange
            let clock = FakeClock::new(fixed_time());
            let clock_ref: &dyn Clock = &clock;

            // act
            let t = clock_ref.now();

            // assert
            assert_eq!(t, fixed_time());
        }
    }
}
