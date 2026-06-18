use super::*;

impl ChatWidget {
    pub(crate) fn open_agent_message_prompt(&mut self, thread_id: ThreadId, context: String) {
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Message agent".to_string(),
            "Write a message for this agent".to_string(),
            String::new(),
            Some(context),
            Box::new(move |message: String| {
                tx.send(AppEvent::SendAgentMessage { thread_id, message });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }

    pub(crate) fn open_agent_followup_prompt(&mut self, thread_id: ThreadId, context: String) {
        let tx = self.app_event_tx.clone();
        let view = CustomPromptView::new(
            "Follow up with agent".to_string(),
            "Write the next task for this agent".to_string(),
            String::new(),
            Some(context),
            Box::new(move |message: String| {
                tx.send(AppEvent::OpenAgentFollowupConfirmation { thread_id, message });
            }),
        );
        self.bottom_pane.show_view(Box::new(view));
    }
}
