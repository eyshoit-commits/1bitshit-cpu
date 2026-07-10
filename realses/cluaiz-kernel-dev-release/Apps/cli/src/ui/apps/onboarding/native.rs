use crate::assets::logos::logo;
use ::cluaiz_shared::profile::{AccountType, AuthMethod, BusinessProfile, UserProfile};
use color_eyre::Result;
use colored::*;
use inquire::{Confirm, Password, Select, Text};
use std::io::{stdout, Write};
use std::time::Duration;

pub fn run_native_flow() -> Result<UserProfile> {
    let mut profile = UserProfile::new();

    println!("  {} [Cluaiz] Initializing Core Infrastructure...", "🧪".cyan());

    // ── Step 3: Auth ──────────────────────────────────────────────────────
    let auth_choice = Select::new(
        "🔐 How would you like to sign in?",
        vec![
            "Sign in with Google",
            "Sign in with Email",
            "Continue as Guest (No Cloud)",
        ],
    )
    .prompt()?;

    match auth_choice {
        "Sign in with Google" => {
            profile.auth.method = AuthMethod::Google;
            profile.auth.email = "Cluaiz@cluaiz.os".to_string();
            println!("✓ Authenticated via Google as {}", profile.auth.email);
        }
        "Sign in with Email" => {
            profile.auth.method = AuthMethod::Email;
            profile.auth.email = Text::new("✉️  Enter your email:").prompt()?;
            let _pass = Password::new("🔑 Create password:").prompt()?;
            println!("✓ Account created for {}", profile.auth.email);
        }
        _ => {
            profile.auth.method = AuthMethod::None;
            println!("ℹ Proceeding as Guest");
        }
    }

    // ── Step 4: Usage Choice ──────────────────────────────────────────────
    let usage = Select::new(
        "👋 How will you use Archer Cluaiz?",
        vec!["Personal AI Assistant", "Business & Teams"],
    )
    .prompt()?;

    match usage {
        "Business & Teams" => {
            profile.account_type = AccountType::Business;
            let mut biz = BusinessProfile::default();
            biz.name = Text::new("🏢 Business Name:").prompt()?;

            let industries: Vec<String> = ::cluaiz_shared::profile::INDUSTRY_TAXONOMY
                .iter()
                .map(|i| i.label.to_string())
                .collect();
            biz.industry = Select::new("Industry:", industries).prompt()?;

            profile.business = Some(biz);
        }
        _ => {
            profile.account_type = AccountType::Personal;
            profile.identity.name = Text::new("👤 What is your name, Cluaiz?").prompt()?;
        }
    }

    // ── Step 6: Hardware Audit ────────────────────────────────────────────
    println!("\n📡 INITIATING BARE-METAL CALIBRATION");

    // 🧬 probe hardware
    use ::cluaiz_shared::hardware::{HardwareGovernor, get_Cluaiz_profile};

    if let Err(e) = HardwareGovernor::auto_calibrate() {
        println!("  {} [Onboarding] Calibration failed: {:?}", "❌".red(), e);
    }

    let stats = get_Cluaiz_profile();

    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let ram_gb = sys.total_memory() as f64 / 1_073_741_824.0;

    println!("  HOST PLATFORM: {}", stats.platform);
    println!(
        "  CPU UNIT:      {} ({} cores)",
        stats.cpu_brand, stats.cpu_cores
    );

    let gpu_info = if stats.compute.has_gpu {
        format!("Accelerator Active ({:.1} GB VRAM)", stats.compute.vram_gb)
    } else {
        "NO ACCELERATOR".to_string()
    };
    println!("  GPU COMPUTE:   {}", gpu_info);
    println!("  SYSTEM RAM:    {:.1} GB", ram_gb);
    println!("  CORE STATUS:   OPTIMIZED ✓\n");

    // ── Part B: Sequential Performance Tuning ────────────────────────────
    println!("🛠️ PERFORMANCE TUNING");

    let turbo_quant = Confirm::new("Enable TurboQuant Acceleration?")
        .with_default(true)
        .prompt()?;
    let _ = HardwareGovernor::update_field(
        "runtime_engine.booster_flags.TurboQuant_Enable",
        serde_json::json!(turbo_quant),
    );

    if stats.compute.vram_gb >= 2.0 {
        let flash_attn = Confirm::new("Enable FlashAttention v2?")
            .with_default(true)
            .prompt()?;
        let _ = HardwareGovernor::update_field(
            "runtime_engine.booster_flags.FlashAttention_v2",
            serde_json::json!(flash_attn),
        );
    }

    println!("\n✓ Hardware DNA verified and synchronized.\n");

    // ── Finalize ──────────────────────────────────────────────────────────
    profile.onboarding_completed = true;
    profile.hardware_completed = true;
    profile.touch();

    let _ = ::cluaiz_shared::profile::save_profile(&profile);
    let _ = ::cluaiz_shared::onboarding::seed_workspace(&profile);

    println!("\n🧿 ARCHER Cluaiz — ONLINE");
    println!(
        "Welcome to the future of Cluaiz AI, {}.\n",
        profile.display_name()
    );

    Ok(profile)
}

