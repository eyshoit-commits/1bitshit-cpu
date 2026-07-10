use crate::ffi::llama_cpp;

/// 🦅 Generate Speculative Drafts (Multi-scale Fallback Prompt Lookup)
pub unsafe fn generate_drafts(
    history: &[i32],
    vocab: *const std::ffi::c_void,
    is_lookahead: bool,
    is_injecting: bool,
    injected_tokens_queue_is_empty: bool,
    n_ctx: u32,
    n_cur: i32,
    lookahead_logs: &mut Vec<String>,
) -> Vec<i32> {
    let mut drafts = Vec::new();
    
    if is_lookahead && history.len() >= 4 && !is_injecting && injected_tokens_queue_is_empty {
        let len = history.len();
        'ngram_loop: for ngram_size in (3..=5).rev() {
            if len < ngram_size + 1 {
                continue;
            }
            let ngram = &history[len - ngram_size..len];
            let max_search_idx = len - ngram_size - 1;
            for i in (0..=max_search_idx).rev() {
                if &history[i..i + ngram_size] == ngram {
                     let mut j = i + ngram_size;
                     while j < len && drafts.len() < 4 {
                         let tok = history[j];
                         if llama_cpp::llama_vocab_is_eog(vocab, tok) {
                             break;
                         }
                         drafts.push(tok);
                         j += 1;
                     }
                    if !drafts.is_empty() {
                        lookahead_logs.push(format!(
                            "🔍 Match found at ngram_size {}: {:?} -> drafts {:?}",
                            ngram_size, ngram, drafts
                        ));
                        break 'ngram_loop;
                    }
                }
            }
        }
    }

    // Ensure drafts don't exceed remaining context space
    let max_drafts = (n_ctx as i32 - n_cur - 2).max(0);
    if drafts.len() > max_drafts as usize {
        drafts.truncate(max_drafts as usize);
    }

    drafts
}
