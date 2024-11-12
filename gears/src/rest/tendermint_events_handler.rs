// TODO: update errors
use anyhow::anyhow;

#[derive(Debug)]
pub struct StrEventsHandler<'a> {
    events_str: &'a str,
}

impl<'a> StrEventsHandler<'a> {
    pub fn new(events_str: &'a str) -> Self {
        let events_str = events_str.trim_start_matches('\'').trim_end_matches('\'');
        Self { events_str }
    }

    pub fn try_parse_tendermint_events_vec(&self) -> anyhow::Result<Vec<String>> {
        let events = self.events_str.split('&');

        let mut tm_events = Vec::with_capacity(self.events_str.matches('&').count() + 1);
        for event in events {
            let tokens = event
                .split_once('=')
                .ok_or(anyhow!("invalid event; event {event} should be of the format: {{eventType}}.{{eventAttribute}}={{value}}"))?;
            if tokens.0 == "tx.height" {
                tm_events.push(format!("{}={}", tokens.0, tokens.1));
            } else {
                tm_events.push(format!("{}='{}'", tokens.0, tokens.1));
            }
        }
        Ok(tm_events)
    }
}
