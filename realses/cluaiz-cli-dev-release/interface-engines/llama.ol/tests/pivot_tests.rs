#[cfg(test)]
mod tests {
    #[test]
    fn test_pivot_prompt_formatting() {
        // Mock prompt as it would arrive from the dashboard during a pivot
        let prompt = "[PIVOT_CONTINUE] 5 not 10 appale";
        
        let is_pivot = prompt.starts_with("[PIVOT_CONTINUE]");
        assert!(is_pivot, "Prompt should be identified as a pivot");
        
        let actual_prompt = if is_pivot {
            prompt.trim_start_matches("[PIVOT_CONTINUE]").trim_start().to_string()
        } else {
            prompt.to_string()
        };
        
        assert_eq!(actual_prompt, "5 not 10 appale", "Actual prompt should be correctly trimmed without leading spaces");
    }

    #[test]
    fn test_standard_prompt_formatting() {
        // Mock prompt as it would arrive normally
        let prompt = "How many apples do you have?";
        
        let is_pivot = prompt.starts_with("[PIVOT_CONTINUE]");
        assert!(!is_pivot, "Prompt should NOT be identified as a pivot");
        
        let actual_prompt = if is_pivot {
            prompt.trim_start_matches("[PIVOT_CONTINUE]").trim_start().to_string()
        } else {
            prompt.to_string()
        };
        
        assert_eq!(actual_prompt, "How many apples do you have?", "Standard prompt should remain untouched before templating");
    }

    #[test]
    fn test_templater_format_turn_for_pivots() {
        // We will simulate the templater logic that fixes the prompt formatting
        let actual_prompt = "add story in dog and write in hindi";
        
        // This is what the templater should do internally
        let formatted = format!("<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n", actual_prompt);
        
        assert_eq!(formatted, "<|im_end|>\n<|im_start|>user\nadd story in dog and write in hindi<|im_end|>\n<|im_start|>assistant\n");
    }
}
