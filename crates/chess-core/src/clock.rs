use shakmaty::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClockMode {
    Fischer,
    Bronstein,
    Hourglass,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Clock {
    remaining_ms: [u64; 2],
    bonus_ms: u64,
    delay_left_ms: u64,
    mode: ClockMode,
    active: Color,
    running: bool,
}

impl Clock {
    pub fn new(minutes: u32, increment_seconds: u32) -> Result<Self, String> {
        Self::new_with_mode(minutes, increment_seconds, ClockMode::Fischer)
    }

    pub fn new_with_mode(
        minutes: u32,
        bonus_seconds: u32,
        mode: ClockMode,
    ) -> Result<Self, String> {
        if !(1..=180).contains(&minutes) || bonus_seconds > 60 {
            return Err("Clock must be 1–180 minutes with 0–60 seconds bonus".into());
        }
        let bonus_ms = if mode == ClockMode::Hourglass {
            0
        } else {
            u64::from(bonus_seconds) * 1_000
        };
        Ok(Self {
            remaining_ms: [u64::from(minutes) * 60_000; 2],
            bonus_ms,
            delay_left_ms: if mode == ClockMode::Bronstein {
                bonus_ms
            } else {
                0
            },
            mode,
            active: Color::White,
            running: true,
        })
    }

    pub fn tick(&mut self, elapsed_ms: u64) -> Option<Color> {
        if !self.running {
            return None;
        }
        let charge = if self.mode == ClockMode::Bronstein {
            let free = elapsed_ms.min(self.delay_left_ms);
            self.delay_left_ms -= free;
            elapsed_ms - free
        } else {
            elapsed_ms
        };
        let index = color_index(self.active);
        if self.mode == ClockMode::Hourglass {
            let transferred = charge.min(self.remaining_ms[index]);
            self.remaining_ms[1 - index] = self.remaining_ms[1 - index]
                .saturating_add(transferred)
                .min(86_400_000);
        }
        if charge >= self.remaining_ms[index] {
            self.remaining_ms[index] = 0;
            self.running = false;
            return Some(self.active);
        }
        self.remaining_ms[index] -= charge;
        None
    }

    pub fn moved(&mut self, mover: Color) {
        if self.running && self.active == mover {
            if self.mode == ClockMode::Fischer {
                let index = color_index(mover);
                self.remaining_ms[index] = self.remaining_ms[index]
                    .saturating_add(self.bonus_ms)
                    .min(86_400_000);
            }
            self.active = !mover;
            self.delay_left_ms = if self.mode == ClockMode::Bronstein {
                self.bonus_ms
            } else {
                0
            };
        }
    }

    pub fn pause(&mut self) {
        self.running = false;
    }
    pub fn resume(&mut self) {
        if self.remaining_ms[color_index(self.active)] > 0 {
            self.running = true;
        }
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn remaining_ms(&self, color: Color) -> u64 {
        self.remaining_ms[color_index(color)]
    }
    pub fn active(&self) -> Color {
        self.active
    }
    pub fn mode(&self) -> ClockMode {
        self.mode
    }
    pub fn bonus_seconds(&self) -> u32 {
        (self.bonus_ms / 1_000) as u32
    }

    pub fn save_text(&self) -> String {
        format!(
            "clock {} {} {} {} {} {} {}",
            self.remaining_ms[0],
            self.remaining_ms[1],
            self.bonus_ms,
            if self.active == Color::White {
                "w"
            } else {
                "b"
            },
            u8::from(self.running),
            match self.mode {
                ClockMode::Fischer => "f",
                ClockMode::Bronstein => "b",
                ClockMode::Hourglass => "h",
            },
            self.delay_left_ms
        )
    }

    pub fn from_save_text(line: &str) -> Result<Self, String> {
        let parts: Vec<_> = line.split_whitespace().collect();
        if (parts.len() != 6 && parts.len() != 8) || parts[0] != "clock" {
            return Err("Invalid clock save".into());
        }
        let parse = |index: usize| {
            parts[index]
                .parse::<u64>()
                .map_err(|_| "Invalid clock value".to_string())
        };
        let remaining_ms = [parse(1)?, parse(2)?];
        let bonus_ms = parse(3)?;
        if remaining_ms.iter().any(|&ms| ms > 86_400_000) || bonus_ms > 60_000 {
            return Err("Clock value out of range".into());
        }
        let active = match parts[4] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err("Invalid active side".into()),
        };
        let running = match parts[5] {
            "0" => false,
            "1" => true,
            _ => return Err("Invalid clock state".into()),
        };
        let mode = if parts.len() == 6 {
            ClockMode::Fischer
        } else {
            match parts[6] {
                "f" => ClockMode::Fischer,
                "b" => ClockMode::Bronstein,
                "h" => ClockMode::Hourglass,
                _ => return Err("Invalid clock mode".into()),
            }
        };
        let delay_left_ms = if parts.len() == 6 { 0 } else { parse(7)? };
        if delay_left_ms > bonus_ms
            || (mode != ClockMode::Bronstein && delay_left_ms != 0)
            || (mode == ClockMode::Hourglass && bonus_ms != 0)
        {
            return Err("Invalid clock delay".into());
        }
        Ok(Self {
            remaining_ms,
            bonus_ms,
            delay_left_ms,
            mode,
            active,
            running,
        })
    }
}

fn color_index(color: Color) -> usize {
    if color == Color::White { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fischer_increment_and_timeout() {
        let mut clock = Clock::new(1, 2).unwrap();
        assert_eq!(clock.tick(10_000), None);
        clock.moved(Color::White);
        assert_eq!(clock.remaining_ms(Color::White), 52_000);
        assert_eq!(clock.active(), Color::Black);
        assert_eq!(clock.tick(60_000), Some(Color::Black));
        assert!(!clock.running());
    }
    #[test]
    fn save_roundtrip() {
        let mut clock = Clock::new(5, 3).unwrap();
        clock.tick(3_000);
        assert_eq!(Clock::from_save_text(&clock.save_text()).unwrap(), clock);
        assert!(Clock::from_save_text("clock 1 2 3 w bad").is_err());
        assert_eq!(
            Clock::from_save_text("clock 60000 60000 2000 w 1")
                .unwrap()
                .mode(),
            ClockMode::Fischer
        );
    }

    #[test]
    fn bronstein_delay_resets_each_turn_and_survives_save() {
        let mut clock = Clock::new_with_mode(1, 3, ClockMode::Bronstein).unwrap();
        assert_eq!(clock.tick(2_000), None);
        assert_eq!(clock.remaining_ms(Color::White), 60_000);
        assert_eq!(Clock::from_save_text(&clock.save_text()).unwrap(), clock);
        assert_eq!(clock.tick(2_000), None);
        assert_eq!(clock.remaining_ms(Color::White), 59_000);
        clock.moved(Color::White);
        assert_eq!(clock.tick(3_000), None);
        assert_eq!(clock.remaining_ms(Color::Black), 60_000);
    }

    #[test]
    fn hourglass_transfers_only_time_spent() {
        let mut clock = Clock::new_with_mode(1, 0, ClockMode::Hourglass).unwrap();
        assert_eq!(clock.tick(10_000), None);
        assert_eq!(clock.remaining_ms(Color::White), 50_000);
        assert_eq!(clock.remaining_ms(Color::Black), 70_000);
        clock.moved(Color::White);
        assert_eq!(clock.tick(5_000), None);
        assert_eq!(clock.remaining_ms(Color::White), 55_000);
        assert_eq!(clock.remaining_ms(Color::Black), 65_000);
        assert_eq!(Clock::from_save_text(&clock.save_text()).unwrap(), clock);
        assert_eq!(clock.tick(70_000), Some(Color::Black));
        assert_eq!(clock.remaining_ms(Color::White), 120_000);
    }
}
