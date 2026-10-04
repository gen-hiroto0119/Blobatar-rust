use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackRate {
    Quarter,
    Half,
    #[default]
    Normal,
    Double,
}

impl PlaybackRate {
    fn factor(self) -> f64 {
        match self {
            Self::Quarter => 0.25,
            Self::Half => 0.5,
            Self::Normal => 1.0,
            Self::Double => 2.0,
        }
    }
}

/// Absolute wall-time anchors avoid accumulating a per-frame rounding error.
#[derive(Clone, Debug, Default)]
pub struct PlaybackClock {
    wall_origin: Duration,
    time_origin_ms: f64,
    rate: PlaybackRate,
    paused: bool,
}

impl PlaybackClock {
    pub fn at(&self, wall: Duration) -> f64 {
        let elapsed = if self.paused {
            0.0
        } else {
            wall.saturating_sub(self.wall_origin).as_secs_f64() * 1000.0 * self.rate.factor()
        };
        self.time_origin_ms + elapsed
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool, wall: Duration) {
        self.reanchor(wall);
        self.paused = paused;
    }

    pub fn set_rate(&mut self, rate: PlaybackRate, wall: Duration) {
        self.reanchor(wall);
        self.rate = rate;
    }

    pub fn seek(&mut self, time: Duration, wall: Duration) {
        self.wall_origin = wall;
        self.time_origin_ms = time.as_secs_f64() * 1000.0;
    }

    fn reanchor(&mut self, wall: Duration) {
        self.time_origin_ms = self.at(wall);
        self.wall_origin = wall;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn pause_rate_and_seek_preserve_timeline_positions() {
        let mut clock = PlaybackClock::default();
        assert_eq!(clock.at(ms(1000)), 1000.0);
        clock.set_rate(PlaybackRate::Quarter, ms(1000));
        assert_eq!(clock.at(ms(3000)), 1500.0);
        clock.set_paused(true, ms(3000));
        assert_eq!(clock.at(ms(8000)), 1500.0);
        clock.set_rate(PlaybackRate::Half, ms(8000));
        clock.seek(ms(1234), ms(8000));
        assert_eq!(clock.at(ms(9000)), 1234.0);
        clock.set_paused(false, ms(9000));
        assert_eq!(clock.at(ms(10000)), 1734.0);
        clock.set_rate(PlaybackRate::Double, ms(10000));
        assert_eq!(clock.at(ms(10100)), 1934.0);
    }
}
