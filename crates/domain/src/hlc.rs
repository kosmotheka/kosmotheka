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
            self.last_millis = now_millis;
            self.last_counter = 0;
        } else {
            self.last_counter += 1;
        }
        self.current()
    }

    /// Timestamp after receiving a remote event: later than both our last
    /// timestamp and the remote one, so causality is preserved across devices.
    pub fn receive(&mut self, remote: &Hlc, now_millis: u64) -> Hlc {
        let millis = now_millis.max(self.last_millis).max(remote.millis);
        let counter = if millis == self.last_millis && millis == remote.millis {
            self.last_counter.max(remote.counter) + 1
        } else if millis == self.last_millis {
            self.last_counter + 1
        } else if millis == remote.millis {
            remote.counter + 1
        } else {
            0
        };
        self.last_millis = millis;
        self.last_counter = counter;
        self.current()
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

impl FromStr for Hlc {
    type Err = HlcParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(3, '-');
        let millis = parts.next().ok_or(HlcParseError)?;
        let counter = parts.next().ok_or(HlcParseError)?;
        let device = parts.next().ok_or(HlcParseError)?;
        if millis.len() != 16 || counter.len() != 8 || device.is_empty() {
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
            device: "dev".into(),
        };
        let later = Hlc {
            millis: 100,
            counter: 0,
            device: "dev".into(),
        };

        assert_eq!(earlier.to_string(), "0000000000000099-0000000f-dev");
        assert_eq!(earlier.to_string().parse(), Ok(earlier.clone()));
        assert!(earlier.to_string() < later.to_string());
    }

    #[test]
    fn device_ids_with_hyphens_survive_parsing() {
        let hlc: Hlc = "0000000000000001-00000000-0199-abcd".parse().unwrap();
        assert_eq!(hlc.device, "0199-abcd");
    }
}
