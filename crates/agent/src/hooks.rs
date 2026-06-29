use std::sync::Arc;

use acp_thread::UserMessageId;
use anyhow::Result;
use gpui::{SharedString, Task};
use language_model::{
    CompletionIntent, LanguageModelId, LanguageModelProviderId, LanguageModelRequest,
    LanguageModelToolResult, LanguageModelToolUseId,
};

#[derive(Clone, Debug)]
pub struct AgentHookContext {
    pub thread_id: SharedString,
    pub prompt_id: SharedString,
}

#[derive(Clone, Debug)]
pub struct SessionHookContext {
    pub agent: AgentHookContext,
    pub user_message_id: Option<UserMessageId>,
}

#[derive(Clone, Debug)]
pub struct LlmCallHookContext {
    pub agent: AgentHookContext,
    pub intent: CompletionIntent,
    pub model_id: LanguageModelId,
    pub provider_id: LanguageModelProviderId,
}

#[derive(Clone, Debug)]
pub struct ToolHookContext {
    pub agent: AgentHookContext,
    pub tool_use_id: LanguageModelToolUseId,
    pub tool_name: Arc<str>,
}

#[derive(Clone, Debug)]
pub enum HookFlow {
    Continue,
    Abort { message: SharedString },
}

#[derive(Clone, Debug)]
pub enum LlmRequestHookFlow {
    Continue(LanguageModelRequest),
    Abort { message: SharedString },
}

pub trait AgentHook {
    fn before_session(&self, _context: SessionHookContext) -> Task<Result<HookFlow>> {
        Task::ready(Ok(HookFlow::Continue))
    }

    fn after_session(&self, _context: SessionHookContext) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn before_llm_call(
        &self,
        _context: LlmCallHookContext,
        request: LanguageModelRequest,
    ) -> Task<Result<LlmRequestHookFlow>> {
        Task::ready(Ok(LlmRequestHookFlow::Continue(request)))
    }

    fn after_llm_call(&self, _context: LlmCallHookContext) -> Task<Result<HookFlow>> {
        Task::ready(Ok(HookFlow::Continue))
    }

    fn before_tool(&self, _context: ToolHookContext) -> Task<Result<HookFlow>> {
        Task::ready(Ok(HookFlow::Continue))
    }

    fn after_tool(
        &self,
        _context: ToolHookContext,
        result: LanguageModelToolResult,
    ) -> Task<Result<LanguageModelToolResult>> {
        Task::ready(Ok(result))
    }
}

#[derive(Clone, Default)]
pub struct AgentHooks {
    hooks: Arc<Vec<Arc<dyn AgentHook>>>,
}

impl AgentHooks {
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    pub fn push(&mut self, hook: Arc<dyn AgentHook>) {
        Arc::make_mut(&mut self.hooks).push(hook);
    }

    pub async fn before_session(&self, context: SessionHookContext) -> Result<HookFlow> {
        for hook in self.hooks.iter() {
            match hook.before_session(context.clone()).await? {
                HookFlow::Continue => {}
                abort @ HookFlow::Abort { .. } => return Ok(abort),
            }
        }
        Ok(HookFlow::Continue)
    }

    pub async fn after_session(&self, context: SessionHookContext) -> Result<()> {
        for hook in self.hooks.iter() {
            hook.after_session(context.clone()).await?;
        }
        Ok(())
    }

    pub async fn before_llm_call(
        &self,
        context: LlmCallHookContext,
        request: LanguageModelRequest,
    ) -> Result<LlmRequestHookFlow> {
        let mut request = request;
        for hook in self.hooks.iter() {
            match hook.before_llm_call(context.clone(), request).await? {
                LlmRequestHookFlow::Continue(next_request) => request = next_request,
                abort @ LlmRequestHookFlow::Abort { .. } => return Ok(abort),
            }
        }
        Ok(LlmRequestHookFlow::Continue(request))
    }

    pub async fn after_llm_call(&self, context: LlmCallHookContext) -> Result<HookFlow> {
        for hook in self.hooks.iter() {
            match hook.after_llm_call(context.clone()).await? {
                HookFlow::Continue => {}
                abort @ HookFlow::Abort { .. } => return Ok(abort),
            }
        }
        Ok(HookFlow::Continue)
    }

    pub async fn before_tool(&self, context: ToolHookContext) -> Result<HookFlow> {
        for hook in self.hooks.iter() {
            match hook.before_tool(context.clone()).await? {
                HookFlow::Continue => {}
                abort @ HookFlow::Abort { .. } => return Ok(abort),
            }
        }
        Ok(HookFlow::Continue)
    }

    pub async fn after_tool(
        &self,
        context: ToolHookContext,
        result: LanguageModelToolResult,
    ) -> Result<LanguageModelToolResult> {
        let mut result = result;
        for hook in self.hooks.iter() {
            result = hook.after_tool(context.clone(), result).await?;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use parking_lot::Mutex;

    struct RecordingHook {
        name: &'static str,
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl AgentHook for RecordingHook {
        fn before_session(&self, _context: SessionHookContext) -> Task<Result<HookFlow>> {
            self.events.lock().push(self.name);
            Task::ready(Ok(HookFlow::Continue))
        }
    }

    struct AbortingHook {
        name: &'static str,
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl AgentHook for AbortingHook {
        fn before_session(&self, _context: SessionHookContext) -> Task<Result<HookFlow>> {
            self.events.lock().push(self.name);
            Task::ready(Ok(HookFlow::Abort {
                message: "blocked".into(),
            }))
        }
    }

    struct RequestPromptHook;

    impl AgentHook for RequestPromptHook {
        fn before_llm_call(
            &self,
            _context: LlmCallHookContext,
            mut request: LanguageModelRequest,
        ) -> Task<Result<LlmRequestHookFlow>> {
            request.prompt_id = Some("modified".to_string());
            Task::ready(Ok(LlmRequestHookFlow::Continue(request)))
        }
    }

    fn session_context() -> SessionHookContext {
        SessionHookContext {
            agent: AgentHookContext {
                thread_id: "thread".into(),
                prompt_id: "prompt".into(),
            },
            user_message_id: None,
        }
    }

    #[gpui::test]
    async fn test_hooks_run_in_registration_order(_cx: &mut TestAppContext) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = AgentHooks::default();
        hooks.push(Arc::new(RecordingHook {
            name: "first",
            events: events.clone(),
        }));
        hooks.push(Arc::new(RecordingHook {
            name: "second",
            events: events.clone(),
        }));

        hooks.before_session(session_context()).await.unwrap();

        assert_eq!(&*events.lock(), &["first", "second"]);
    }

    #[gpui::test]
    async fn test_hook_abort_stops_later_hooks(_cx: &mut TestAppContext) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut hooks = AgentHooks::default();
        hooks.push(Arc::new(AbortingHook {
            name: "first",
            events: events.clone(),
        }));
        hooks.push(Arc::new(RecordingHook {
            name: "second",
            events: events.clone(),
        }));

        let flow = hooks.before_session(session_context()).await.unwrap();

        assert!(matches!(flow, HookFlow::Abort { .. }));
        assert_eq!(&*events.lock(), &["first"]);
    }

    #[gpui::test]
    async fn test_llm_hook_can_modify_request(_cx: &mut TestAppContext) {
        let mut hooks = AgentHooks::default();
        hooks.push(Arc::new(RequestPromptHook));
        let context = LlmCallHookContext {
            agent: AgentHookContext {
                thread_id: "thread".into(),
                prompt_id: "prompt".into(),
            },
            intent: CompletionIntent::UserPrompt,
            model_id: LanguageModelId("model".into()),
            provider_id: LanguageModelProviderId::new("test"),
        };

        let flow = hooks
            .before_llm_call(context, LanguageModelRequest::default())
            .await
            .unwrap();

        let LlmRequestHookFlow::Continue(request) = flow else {
            panic!("expected modified request");
        };
        assert_eq!(request.prompt_id.as_deref(), Some("modified"));
    }
}
