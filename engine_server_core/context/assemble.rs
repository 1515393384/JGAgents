use crate::context::budget::TokenBudget;
use crate::context::model::{AssembledWindow, CtxMsg, CtxPrefix};
use crate::core::error::EngineError;

pub fn assemble_window(
    prefix: &CtxPrefix,
    history: &[CtxMsg],
    budget: TokenBudget,
) -> Result<AssembledWindow, EngineError> {
    let cap = budget.prompt_cap();
    let prefix_tokens = prefix.total_tokens();
    if prefix_tokens > cap {
        return Err(EngineError::budget("anchored prefix exceeds token budget"));
    }

    let remaining = cap - prefix_tokens;
    let mut kept_rev: Vec<CtxMsg> = Vec::with_capacity(history.len());
    let mut used_history = 0u32;

    for msg in history.iter().rev() {
        let next = used_history.saturating_add(msg.token_estimate);
        if next > remaining {
            break;
        }
        used_history = next;
        kept_rev.push(msg.clone());
    }

    let trimmed_count = history.len().saturating_sub(kept_rev.len());
    kept_rev.reverse();

    let mut messages = Vec::with_capacity(prefix.messages.len() + kept_rev.len());
    messages.extend(prefix.messages.iter().cloned());
    messages.extend(kept_rev);

    Ok(AssembledWindow {
        prefix_version: prefix.version,
        messages,
        estimated_tokens: prefix_tokens.saturating_add(used_history),
        trimmed_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::model::CtxRole;
    use crate::core::error::ErrorCode;

    fn msg(seq: u64, tokens: u32) -> CtxMsg {
        CtxMsg {
            seq,
            role: CtxRole::User,
            content: format!("m{seq}"),
            token_estimate: tokens,
        }
    }

    #[test]
    fn assemble_preserves_prefix_and_trims_oldest_history() {
        let prefix = CtxPrefix {
            version: 2,
            messages: vec![CtxMsg {
                seq: 0,
                role: CtxRole::System,
                content: String::from("sys"),
                token_estimate: 100,
            }],
        };
        let history = vec![msg(1, 50), msg(2, 60), msg(3, 70), msg(4, 80)];
        let budget = TokenBudget::new(300, 50).expect("valid budget"); // cap = 250, rem = 150

        let out = assemble_window(&prefix, &history, budget).expect("assemble");
        assert_eq!(out.prefix_version, 2);
        assert_eq!(out.trimmed_count, 2);
        assert_eq!(out.estimated_tokens, 250);
        assert_eq!(
            out.messages.iter().map(|m| m.seq).collect::<Vec<_>>(),
            vec![0, 3, 4]
        );
    }

    #[test]
    fn assemble_rejects_oversized_prefix() {
        let prefix = CtxPrefix {
            version: 1,
            messages: vec![msg(0, 400)],
        };
        let budget = TokenBudget::new(300, 50).expect("valid budget");
        let err = assemble_window(&prefix, &[], budget).expect_err("must fail");
        assert_eq!(err.code, ErrorCode::Budget);
    }
}
