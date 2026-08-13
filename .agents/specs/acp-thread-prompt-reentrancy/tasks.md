# Implementation Plan: ACP Thread Prompt Reentrancy

## Tasks

- [x] 1. Move prompt invocation outside the `AcpThread` lease and add production-path regression coverage
  - _id: acp-thread-prompt-reentrancy-fix_
  - _priority: P0_
  - _value: high_
  - _wave: 1_
  - _reads: crates/gpui/src/app/entity_map.rs, crates/gpui/src/app.rs, crates/acp_thread/src/acp_thread.rs, crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _writes: crates/acp_thread/src/acp_thread.rs, crates/agent/src/agent.rs_
  - _validation: RUST_BACKTRACE=full SEED=0 cargo test -p agent test_native_local_command_output_does_not_reenter_acp_thread -- --nocapture; ITERATIONS=10 cargo test -p agent test_native_local_command_output_does_not_reenter_acp_thread; cargo test -p acp_thread; cargo test -p agent; cargo test -p agent_ui; cargo fmt --all -- --check; ./script/clippy -p acp_thread -p agent -p agent_ui; python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/acp-thread-prompt-reentrancy --require-complete; git diff --check_
  - _Requirements: 1.1, 1.2, 1.3, 2.1, 2.2, 2.3_
  - Outcome: Native connection prompts begin without an active `AcpThread` lease, so local command output cannot re-enter the entity update while all existing turn behavior remains intact.
  - Design: D1 / prompt dispatch boundary; D2 / existing turn state machine; D3 / production-path regression
  - Done when: The regression fails on the prior boundary, passes after the fix across repeated GPUI scheduler iterations, all affected crate tests and static checks pass, and the specification validates completely.
  - _Evidence: The regression reproduced the prior `double_lease_panic` with a full backtrace, passed for seeds 0 through 9 after the fix, all `acp_thread`, `agent`, and `agent_ui` tests passed, and formatting, clippy, spec validation, and diff checks passed on 2026-08-12._
