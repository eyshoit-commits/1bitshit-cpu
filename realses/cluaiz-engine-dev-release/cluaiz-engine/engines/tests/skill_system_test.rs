// CLUAIZ-OS: Sovereign Skill System - Industrial Integration Test
// Goal: Validate Registry, Router, Security, and Neural Stitching in one flow.

#[cfg(test)]
mod tests {
    use engines::neural_foundry::{NeuralFoundry, IntentResult, security::guard::PermissionLevel};
    use std::fs;
    use std::path::Path;

    /// 🛠️ Setup: Creates a mock skill directory for testing.
    fn setup_mock_skills(base_path: &str) {
        let skill_path = Path::new(base_path).join("communication/whatsapp-test");
        fs::create_dir_all(&skill_path).unwrap();

        let manifest = r#"{
            "id": "cluaiz.skill.whatsapp.test",
            "name": "WhatsApp Test",
            "version": "1.0.0",
            "author": "Cluaiz-Lab",
            "description": "Mock skill for integration testing",
            "triggers": {
                "semantic": ["whatsapp", "chat", "message"],
                "entropy_threshold": 0.5
            },
            "permissions": {
                "level": "ReadOnly",
                "network": false,
                "filesystem": false,
                "mcp_servers": []
            },
            "soul_type": "KV_CACHE",
            "neural_metadata": {
                "token_count": 512,
                "head_dim": 128
            }
        }"#;
        fs::write(skill_path.join("manifest.json"), manifest).unwrap();
        fs::write(skill_path.join("state.kv-cache"), vec![0u8; 1024]).unwrap(); // Dummy cache
        
        // Valid minimal WASM with 'memory' and 'run' function (Verified Bytes)
        let wasm_bytes = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f, 0x7f, 0x01, 
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x05, 0x03, 0x01, 0x00, 0x01, 0x07, 0x11, 0x02, 0x06, 0x6d, 0x65, 
            0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x03, 0x72, 0x75, 0x6e, 0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 
            0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b
        ];
        fs::write(skill_path.join("logic.wasm"), wasm_bytes).unwrap();
    }

    #[tokio::test]
    async fn test_full_neural_foundry_flow() {
        let test_dir = "tests/temp_skills";
        if Path::new(test_dir).exists() {
            fs::remove_dir_all(test_dir).unwrap();
        }
        setup_mock_skills(test_dir);

        let mut foundry = NeuralFoundry::new();
        foundry.initialize(test_dir);

        println!("🧪 [Test] Running intent discovery for 'Send a whatsapp message'...");
        
        // 1. Test Routing & Discovery
        let res = foundry.process_intent("i want to send a whatsapp message to aryan").await;
        
        // We verify that the process reached the end (even if WASM logic failed due to dummy bytes)
        assert!(res.is_ok(), "Foundry process_intent failed: {:?}", res.err());
        let intent_result = res.unwrap();
        
        // ASSERT: Skill should be triggered
        assert!(!intent_result.signals.is_empty(), "❌ Neural Signals should be generated for KV_CACHE skill");
        println!("✅ [Test] Neural Signal generated successfully.");

        // 2. Test Signal Integrity
        let signal = &intent_result.signals[0];
        assert_eq!(signal.token_count, 512, "❌ Signal token count mismatch");
        assert_eq!(signal.head_dim, 128, "❌ Signal head dimension mismatch");

        // 3. Test Security Guard (Positive Case)
        let skill = foundry.registry.skills.first().unwrap();
        let guard_res = foundry.guard.validate_action(&skill.manifest, PermissionLevel::ReadOnly);
        assert!(guard_res.is_ok(), "❌ Security Guard should allow ReadOnly access");
        println!("✅ [Test] Security Guard validated ReadOnly access.");

        // 4. Test Security Guard (Negative Case - Unauthorized Level)
        let guard_fail_res = foundry.guard.validate_action(&skill.manifest, PermissionLevel::DangerFullAccess);
        assert!(guard_fail_res.is_err(), "❌ Security Guard should BLOCK DangerFullAccess for a ReadOnly skill");
        println!("✅ [Test] Security Guard correctly BLOCKED unauthorized access.");

        // Clean up
        fs::remove_dir_all(test_dir).unwrap();
        println!("🏁 [Test] Integration Test Completed Successfully.");
    }
}
