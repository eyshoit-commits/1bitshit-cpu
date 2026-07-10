#[cfg(test)]
mod tests {
    use engines::neural_foundry::NeuralFoundry;
    use std::fs;
    use std::path::PathBuf;

    fn setup_mock_skill(skills_dir: &PathBuf, id: &str) {
        let skill_path = skills_dir.join(id);
        fs::create_dir_all(&skill_path).unwrap();

        let manifest = format!(
            r#"{{
            "id": "{}",
            "name": "Skill {}",
            "version": "1.0.0",
            "author": "Antigravity",
            "description": "Stress Test Skill",
            "triggers": {{
                "semantic": ["{}"],
                "entropy_threshold": 0.5
            }},
            "permissions": {{
                "level": "ReadOnly",
                "network": false,
                "filesystem": false
            }},
            "soul_type": "KV_CACHE"
        }}"#,
            id, id, id
        );

        fs::write(skill_path.join("manifest.json"), manifest).unwrap();
        fs::write(skill_path.join("state.kv-cache"), vec![0u8; 1024]).unwrap();
    }

    #[tokio::test]
    async fn test_vram_eviction_stress() {
        let temp_dir = std::env::temp_dir().join("cluaiz_stress_test");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).unwrap();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        // 1. Setup 5 mock skills (Limit is 3)
        for i in 1..=5 {
            setup_mock_skill(&temp_dir, &format!("skill_{}", i));
        }

        let mut foundry = NeuralFoundry::new();
        foundry.initialize(temp_dir.to_str().unwrap());

        // 2. Activate skills in sequence
        println!("🚀 Starting Stress Test: Activating 5 skills...");

        for i in 1..=5 {
            let prompt = format!("skill_{}", i);
            let _ = foundry.process_intent(&prompt).await.unwrap();

            let active_ids = foundry.active_skill_ids.lock().unwrap();
            println!("Active Skills (LRU): {:?}", *active_ids);

            // Limit check
            assert!(
                active_ids.len() <= 3,
                "VRAM Guardian failed! Active skills exceeded limit."
            );
        }

        // 3. Verify LRU: skill_1 and skill_2 should have been evicted
        let active_ids = foundry.active_skill_ids.lock().unwrap();
        assert!(!active_ids.contains(&"skill_1".to_string()));
        assert!(!active_ids.contains(&"skill_2".to_string()));
        assert!(active_ids.contains(&"skill_5".to_string()));

        println!(
            "✅ Stress Test Passed: VRAM Guardian successfully evicted least recently used skills."
        );

        // Cleanup
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
