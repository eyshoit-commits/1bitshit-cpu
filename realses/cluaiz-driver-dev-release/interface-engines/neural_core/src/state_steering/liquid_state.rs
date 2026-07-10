//! 🌊 Liquid State Neural Logic
//! Models dynamic time-constants for continuous depth adjustments.

pub struct LiquidStateEngine {
    pub time_constant: f32,
    pub leak_rate: f32,
}

impl LiquidStateEngine {
    pub fn new(time_constant: f32, leak_rate: f32) -> Self {
        Self { time_constant, leak_rate }
    }

    /// 🌊 Applies ordinary differential equations (ODE) approximations to the hidden state
    pub fn step_state(&self, current_state: &mut [f32], input_stimulus: &[f32], delta_t: f32) {
        if current_state.len() != input_stimulus.len() { return; }

        for (state, stimulus) in current_state.iter_mut().zip(input_stimulus.iter()) {
            // Liquid differential approximation: dx/dt = -x/tau + f(x) + I
            let decay = -(*state * self.leak_rate);
            let update = decay + *stimulus;
            *state += (update * delta_t) / self.time_constant;
        }
    }
}
