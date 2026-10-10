//! Hybrid logical clock: timestamps that order events the same way on every device,
//! even when their wall clocks disagree.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Field order matters: the derived `Ord` compares time, then counter, then device.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hlc {
    pub millis: u64,
    pub counter: u32,
    pub device: String,
}

/// How far ahead of this device's clock a received timestamp may be. Beyond it the
/// sender's clock is wrong, and taking its time would let it win every field it
/// touches and drag this clock along for good (DAT-005).
pub const MAX_CLOCK_DRIFT_MILLIS: u64 = 60 * 60 * 1000;

impl Hlc {
    /// Whether this timestamp is further ahead of `now_millis` than any honest clock.
    pub fn is_ahead_of(&self, now_millis: u64) -> bool {
        self.millis > now_millis.saturating_add(MAX_CLOCK_DRIFT_MILLIS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("malformed HLC timestamp")]
pub struct HlcParseError;

/// Remembers the last timestamp it issued so it never goes backwards.
#[derive(Debug, Clone)]
pub struct HlcClock {
    device: String,
    last_millis: u64,
    last_counter: u32,
}

impl HlcClock {
    pub fn new(device: impl Into<String>, last: Option<&Hlc>) -> Self {
        Self {
            device: device.into(),
            last_millis: last.map_or(0, |hlc| hlc.millis),
            last_counter: last.map_or(0, |hlc| hlc.counter),
        }
    }

    /// Timestamp for a local event. If the wall clock went backwards or has not
    /// moved, the counter keeps timestamps strictly increasing.
    pub fn tick(&mut self, now_millis: u64) -> Hlc {
        if now_millis > self.last_millis {
            self.set(now_millis, None);
        } else {
            self.set(self.last_millis, Some(self.last_counter));
        }
        self.current()
    }

    /// Timestamp after receiving a remote event: later than both our last
    /// timestamp and the remote one, so causality is preserved across devices.
    pub fn receive(&mut self, remote: &Hlc, now_millis: u64) -> Hlc {
        let millis = now_millis.max(self.last_millis).max(remote.millis);
        let after = if millis == self.last_millis && millis == remote.millis {
            Some(self.last_counter.max(remote.counter))
        } else if millis == self.last_millis {
            Some(self.last_counter)
        } else if millis == remote.millis {
            Some(remote.counter)
        } else {
            None
        };
        self.set(millis, after);
        self.current()
    }

    /// Moves to `millis`, with the counter just past `after`. A full counter moves
    /// on to the next millisecond instead of wrapping to 0, which would go
    /// backwards (SEG-020).
    fn set(&mut self, millis: u64, after: Option<u32>) {
        (self.last_millis, self.last_counter) = match after.map(|c| c.checked_add(1)) {
            None => (millis, 0),
            Some(Some(counter)) => (millis, counter),
            Some(None) => (millis + 1, 0),
        };
    }

    fn current(&self) -> Hlc {
        Hlc {
            millis: self.last_millis,
            counter: self.last_counter,
            device: self.device.clone(),
        }
    }
}

/// Text form `0001727222222222-0000000a-<device>`: zero padding makes text order
/// match timestamp order, so the database can sort on the column directly.
impl fmt::Display for Hlc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:016}-{:08x}-{}",
            self.millis, self.counter, self.device
        )
    }
}

/// Whether `text` is the id of a device: a UUID written `8-4-4-4-12` in **lowercase** hexadecimal. It is what a
/// device is in every operation, so two writings of one id can never be two devices (SEG-029).
pub fn is_device_id(text: &str) -> bool {
    text.len() == 36
        && text.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
        })
}

impl FromStr for Hlc {
    type Err = HlcParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, '-');
        let millis = parts.next().ok_or(HlcParseError)?;
        let counter = parts.next().ok_or(HlcParseError)?;
        let device = parts.next().ok_or(HlcParseError)?;
        if millis.len() != 16 || counter.len() != 8 || !is_device_id(device) {
            return Err(HlcParseError);
        }
        Ok(Hlc {
            millis: millis.parse().map_err(|_| HlcParseError)?,
            counter: u32::from_str_radix(counter, 16).map_err(|_| HlcParseError)?,
            device: device.to_owned(),
        })
    }
}

impl Serialize for Hlc {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Hlc {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_follows_the_wall_clock_when_it_moves_forward() {
        let mut clock = HlcClock::new("a", None);
        assert_eq!(clock.tick(100).millis, 100);
        let next = clock.tick(200);
        assert_eq!((next.millis, next.counter), (200, 0));
    }

    #[test]
    fn tick_never_goes_backwards_when_the_wall_clock_does() {
        let mut clock = HlcClock::new("a", None);
        let first = clock.tick(500);
        let second = clock.tick(300);
        let third = clock.tick(500);
        assert_eq!((second.millis, second.counter), (500, 1));
        assert_eq!((third.millis, third.counter), (500, 2));
        assert!(first < second && second < third);
    }

    #[test]
    fn receive_jumps_ahead_of_a_remote_clock_that_runs_fast() {
        let mut clock = HlcClock::new("a", None);
        clock.tick(100);
        let remote = Hlc {
            millis: 900,
            counter: 4,
            device: "b".into(),
        };

        let after = clock.receive(&remote, 150);

        assert_eq!((after.millis, after.counter), (900, 5));
        assert!(after > remote);
        assert!(
            clock.tick(160) > after,
            "later local events stay after the remote one"
        );
    }

    #[test]
    fn a_full_counter_moves_on_to_the_next_millisecond() {
        let last = Hlc {
            millis: 1_000,
            counter: u32::MAX,
            device: "a".into(),
        };
        let mut clock = HlcClock::new("a", Some(&last));

        let next = clock.tick(500);

        assert!(next > last);
        assert_eq!((next.millis, next.counter), (1_001, 0));
    }

    #[test]
    fn receiving_a_full_counter_stays_ahead_of_it() {
        let mut clock = HlcClock::new("a", None);
        let remote = Hlc {
            millis: 2_000,
            counter: u32::MAX,
            device: "b".into(),
        };

        let next = clock.receive(&remote, 1_000);

        assert!(next > remote);
        assert_eq!((next.millis, next.counter), (2_001, 0));
    }

    #[test]
    fn a_timestamp_more_than_the_allowed_drift_ahead_is_from_the_future() {
        let now = 10_000_000;
        let at = |millis| Hlc {
            millis,
            counter: 0,
            device: "b".into(),
        };

        assert!(!at(now + MAX_CLOCK_DRIFT_MILLIS).is_ahead_of(now));
        assert!(at(now + MAX_CLOCK_DRIFT_MILLIS + 1).is_ahead_of(now));
    }

    #[test]
    fn resumes_from_the_last_persisted_timestamp() {
        let last = Hlc {
            millis: 1_000,
            counter: 7,
            device: "a".into(),
        };
        let mut clock = HlcClock::new("a", Some(&last));
        let next = clock.tick(999);
        assert_eq!((next.millis, next.counter), (1_000, 8));
    }

    #[test]
    fn text_form_round_trips_and_sorts_like_the_struct() {
        let earlier = Hlc {
            millis: 99,
            counter: 15,
            device: "01997c7e-0000-7000-8000-0000000000ff".into(),
        };
        let later = Hlc {
            millis: 100,
            counter: 0,
            device: "01997c7e-0000-7000-8000-0000000000ff".into(),
        };

        assert_eq!(
            earlier.to_string(),
            "0000000000000099-0000000f-01997c7e-0000-7000-8000-0000000000ff"
        );
        assert_eq!(earlier.to_string().parse(), Ok(earlier.clone()));
        assert!(earlier.to_string() < later.to_string());
    }

    #[test]
    fn device_ids_with_hyphens_survive_parsing() {
        let hlc: Hlc = "0000000000000001-00000000-01997c7e-0000-7000-8000-0000000000ff"
            .parse()
            .unwrap();
        assert_eq!(hlc.device, "01997c7e-0000-7000-8000-0000000000ff");
    }

    #[test]
    fn the_allowed_drift_is_one_hour() {
        let now = 5_000_000_000;
        let at = |millis| Hlc {
            millis,
            counter: 0,
            device: "b".into(),
        };
        assert!(!at(now + 3_600_000).is_ahead_of(now));
        assert!(at(now + 3_600_001).is_ahead_of(now));
    }

    #[test]
    fn receive_takes_the_counter_of_whichever_clock_decides_the_millisecond() {
        let remote = |millis, counter| Hlc {
            millis,
            counter,
            device: "b".into(),
        };
        let clock_at = |millis, counter| {
            HlcClock::new(
                "a",
                Some(&Hlc {
                    millis,
                    counter,
                    device: "a".into(),
                }),
            )
        };
        let result = |mut clock: HlcClock, remote: &Hlc| {
            let hlc = clock.receive(remote, 10);
            (hlc.millis, hlc.counter)
        };

        // Same millisecond on both: the larger counter, plus one.
        assert_eq!(result(clock_at(100, 3), &remote(100, 7)), (100, 8));
        assert_eq!(result(clock_at(100, 7), &remote(100, 3)), (100, 8));
        // Only ours is the latest: the remote counter does not count.
        assert_eq!(result(clock_at(100, 3), &remote(50, 9)), (100, 4));
        // Only the remote is the latest: our counter does not count.
        assert_eq!(result(clock_at(50, 9), &remote(100, 3)), (100, 4));
        // The wall clock is the latest: the counter starts over.
        assert_eq!(result(clock_at(5, 9), &remote(8, 9)), (10, 0));
    }

    #[test]
    fn a_malformed_text_form_is_rejected_whichever_part_is_wrong() {
        let valid = "0000000000000001-00000000-01997c7e-0000-7000-8000-0000000000ff";
        assert!(valid.parse::<Hlc>().is_ok());
        for bad in [
            "000000000000001-00000000-dev", // 15 digits of time
            "00000000000000001-00000000-01997c7e-0000-7000-8000-0000000000ff", // 17 digits of time
            "0000000000000001-0000000-01997c7e-0000-7000-8000-0000000000ff", // 7 digits of counter
            "0000000000000001-000000000-01997c7e-0000-7000-8000-0000000000ff", // 9 digits of counter
            "0000000000000001-00000000-",                                      // no device
            "0000000000000001-00000000-dev",                                   // not a UUID
            "0000000000000001-00000000-01997C7E-0000-7000-8000-0000000000FF",  // capitals (SEG-029)
            "0000000000000001-00000000-01997c7e00007000800000000000ff",        // no hyphens
            "0000000000000001-00000000-01997c7e-0000-7000-8000-0000000000fg",  // not hexadecimal
        ] {
            assert_eq!(bad.parse::<Hlc>(), Err(HlcParseError), "{bad}");
        }
    }
}
