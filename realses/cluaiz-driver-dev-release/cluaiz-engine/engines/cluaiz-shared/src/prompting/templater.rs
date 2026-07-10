use std::collections::HashMap;

/// 🎭 TemplateManager: Handles neural prompt formatting based on model DNA.
#[derive(Debug, Clone)]
pub struct TemplateManager {
    pub templates: HashMap<String, String>,
}

impl Default for TemplateManager {
    fn default() -> Self {
        let mut templates = HashMap::new();
        templates.insert("llama".into(), "[INST] {{prompt}} [/INST]".into());
        templates.insert("gemma".into(), "<start_of_turn>user\n{{prompt}}<end_of_turn>\n<start_of_turn>model\n".into());
        Self { templates }
    }
}

impl TemplateManager {
    pub fn format(&self, arch: &str, prompt: &str) -> String {
        self.templates.get(arch)
            .map(|t| t.replace("{{prompt}}", prompt))
            .unwrap_or_else(|| prompt.to_string())
    }
}
