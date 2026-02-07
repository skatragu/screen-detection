use crate::agent::agent_model::{AgentAction, AgentMemory};

#[derive(Debug)]
pub enum BudgetDecision {
    Allow,
    Block(&'static str),
}

pub fn check_budgets(memory: &AgentMemory, proposed: &AgentAction) -> BudgetDecision {
    // ---- Think budget ----
    if memory.think_budget_remaining == 0 {
        return BudgetDecision::Block("think_budget_exhausted");
    }

    // ---- Retry budget ----
    if let Some(last) = &memory.last_action {
        if last == proposed && memory.retry_budget_remaining == 0 {
            return BudgetDecision::Block("retry_budget_exhausted");
        }
    }

    // ---- Loop budget ----
    if memory.loop_budget_remaining == 0 {
        return BudgetDecision::Block("loop_budget_exhausted");
    }

    BudgetDecision::Allow
}
