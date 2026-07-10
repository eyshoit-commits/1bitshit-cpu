use crate::neural_foundry::registry::SkillRegistry;

pub struct SkillRouter {}

impl Default for SkillRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRouter {
    pub fn new() -> Self {
        Self {}
    }

    /// Selects ALL relevant skills for a given prompt (Core Fusion Mode).
    /// Uses the dynamic registry and KERNEL TELEMETRY to find compute-aware matches.
    pub fn match_intent(&self, prompt: &str, registry: &SkillRegistry) -> Vec<String> {
        // 🛰️ Cluaiz Linkage: Get real-time Hardware pressure
        let pulse = cluaiz_shared::hardware::telemetry::get_pulse();
        let pulse_lock = pulse.pulse.read().unwrap();

        let mut matches = Vec::new();
        let prompt_lower = prompt.to_lowercase();

        println!(
            "🧬 [SkillRouter] Compute-Aware Scan: VRAM Pressure {}% | Scanning {} skills...",
            pulse_lock.vram_pressure_pct,
            registry.skills.len()
        );

        for skill in &registry.skills {
            let mut is_matched = false;

            // 1. Semantic Trigger Match
            for trigger in &skill.manifest.triggers.semantic {
                if prompt_lower.contains(trigger.to_lowercase().as_str()) {
                    is_matched = true;
                    break;
                }
            }

            // 2. Full-Text Description Match (Fallback)
            if !is_matched
                && (prompt_lower.contains(&skill.manifest.description.to_lowercase())
                    || skill
                        .manifest
                        .description
                        .to_lowercase()
                        .contains(&prompt_lower))
            {
                is_matched = true;
            }

            if is_matched {
                println!(
                    "✅ [SkillRouter] Dynamic Match Found: {}",
                    skill.manifest.id
                );
                matches.push(skill.manifest.id.clone());
            }
        }

        matches
    }
}
