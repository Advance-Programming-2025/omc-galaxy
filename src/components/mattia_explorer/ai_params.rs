/// All tunable AI parameters for the explorer.
/// These were previously hardcoded as `const` values in `explorer_ai.rs`.
/// Extracting them into a struct allows runtime configuration and ML-based tuning.
#[derive(Debug, Clone)]
pub struct AiParams {
    // --- NOISE ---
    /// Noise level for utility calculations (multiplier range: [1-val, 1+val])
    pub randomness_range: f64,

    // --- INFORMATION DECAY ---
    /// Exponential decay factor for outdated information: e^(-lambda * delta_t)
    pub lambda: f32,

    // --- RESOURCE NEEDS ---
    /// How much a parent resource's need propagates to child resources
    pub propagation_factor: f32,

    // --- SAFETY THRESHOLDS ---
    /// Critical danger threshold - triggers immediate evacuation
    pub safety_critical: f32,
    /// Warning threshold - start looking for safer planets
    pub safety_warning: f32,
    /// Minimum energy cells to consider a planet "defended"
    pub energy_cells_defense_threshold: u32,

    // --- INFORMATION STALENESS ---
    /// Max age (in ticks) before energy info is considered stale
    pub max_energy_info_age: u64,

    // --- HYSTERESIS ---
    /// Minimum advantage required to switch from the current action
    pub action_hysteresis_margin: f32,

    // --- CHARGE RATE PREDICTIONS ---
    /// Minimum charge rate to consider planet "actively charging"
    pub min_active_charge_rate: f32,
    /// Maximum ticks into future to predict (avoid over-optimistic projections)
    pub max_prediction_horizon: u64,
    /// Ticks within which info is considered perfectly accurate
    pub perfect_info_max_time: u64,

    // --- ESCAPE ---
    /// Minimum safety difference needed to justify fleeing
    pub safety_min_diff: f32,

    // --- UTILITY WEIGHTS (previously inline magic numbers) ---
    /// Base utility for the "wait" action
    pub wait_base: f32,
    /// Bonus utility for "wait" when on a safe, charging planet
    pub wait_bonus: f32,

    // --- SAFETY SCORE WEIGHTS ---
    /// Weight for the sustainability component of safety score
    pub safety_weight_sustainability: f32,
    /// Weight for the physical_safety * rocket component
    pub safety_weight_physical: f32,
    /// Weight for the escape factor component
    pub safety_weight_escape: f32,

    // --- CHARGE RATE EMA ---
    /// Exponential moving average alpha for charge rate calculation
    pub charge_rate_alpha: f32,
}

impl Default for AiParams {
    fn default() -> Self {
        Self {
            randomness_range: 0.1,
            lambda: 0.005,
            propagation_factor: 0.8,
            safety_critical: 0.3,
            safety_warning: 0.6,
            energy_cells_defense_threshold: 2,
            max_energy_info_age: 150,
            action_hysteresis_margin: 0.07,
            min_active_charge_rate: 0.05,
            max_prediction_horizon: 100,
            perfect_info_max_time: 10,
            safety_min_diff: 0.07,
            wait_base: 0.08,
            wait_bonus: 0.1,
            safety_weight_sustainability: 0.15,
            safety_weight_physical: 0.70,
            safety_weight_escape: 0.15,
            charge_rate_alpha: 0.3,
        }
    }
}
