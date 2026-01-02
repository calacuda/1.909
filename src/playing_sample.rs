#[derive(Clone, Copy, PartialOrd, PartialEq)]
pub struct PlayingSample {
    pub sample: (usize, usize),
    pub position: usize,
    sample_len: usize,
    pub gain: f32,
}

impl PlayingSample {
    pub fn new(sample: (usize, usize), sample_len: usize, gain: f32) -> Self {
        Self {
            sample,
            position: 0,
            sample_len,
            gain,
        }
    }

    pub fn is_done(&self) -> bool {
        self.position >= self.sample_len
    }

    pub fn step(&mut self) -> Option<usize> {
        let sample_i = self.position;

        if sample_i >= self.sample_len {
            return None;
        } else {
            self.position += 1;
        }

        Some(sample_i)
    }
}
