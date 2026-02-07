use screen_detection::run_app;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_app();

    // let mut agent = Agent {
    //     state: AgentState::Observe,
    //     memory: AgentMemory::default(),
    //     step: 0,
    // memory: AgentMemory {
    // think_budget_remaining: 5,
    // retry_budget_remaining: 3,
    // loop_budget_remaining: 5,
    // ..Default::default()
    // };

    // loop {
    //     let screen = capture_screen();
    //     let diff = semantic_diff(&screen);

    //     if let Some(action) = agent.step(&screen, &diff, &tracer) {
    //         execute_action(&action, &screen).unwrap();
    //         agent.memory.last_action = Some(action);
    //         agent.memory.attempt_count += 1;
    //     }

    // if agent.memory.last_signal != diff.signals.last().cloned() {
    //     agent.memory.attempt_count = 0;
    //     agent.memory.loop_count = 0;
    // }

    //     if matches!(agent.state, AgentState::Stop) {
    //         break;
    //     }
    // }

    Ok(())
}
