use crate::{
    signing::handler::MetadataGetter,
    types::{
        msg::send::MsgSend,
        rendering::screen::{Indent, Screen},
    },
};

use crate::signing::renderer::value_renderer::{
    DefaultPrimitiveRenderer, PrimitiveValueRenderer, RenderError,
    TryPrimitiveValueRendererWithMetadata, ValueRenderer,
};

impl ValueRenderer for MsgSend {
    /// Format `MsgSend`
    /// Note: This implementation doesn't include `Screen` with information about beginning of message and name
    fn format<MG: MetadataGetter>(&self, get_metadata: &MG) -> Result<Vec<Screen>, RenderError> {
        let screens_vec = vec![
            Screen {
                title: "From address".to_string(),
                content: DefaultPrimitiveRenderer::format(self.from_address.clone()),
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "To address".to_string(),
                content: DefaultPrimitiveRenderer::format(self.to_address.clone()),
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "Amount".to_string(),
                content: DefaultPrimitiveRenderer::try_format_with_metadata(
                    self.amount.to_owned(),
                    get_metadata,
                )?,
                indent: Some(Indent::two()),
                expert: false,
            },
        ];

        Ok(screens_vec)
    }
}

#[cfg(test)]
mod tests {
    use crate::signing::renderer::test_functions::TestMetadataGetter;
    use crate::types::{msg::send::MsgSend, rendering::screen::Screen};

    use crate::signing::renderer::value_renderer::ValueRenderer;

    #[test]
    fn msg_send_multiple_coins() -> anyhow::Result<()> {
        const MESSAGE: &str = r#"{
            "from_address": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs",
            "to_address": "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t",
            "amount": [{ "denom": "uatom", "amount": "10000000" }, { "denom": "ucosm", "amount": "10000000"}]
        }"#;

        let msg: MsgSend = serde_json::from_str(MESSAGE)?;

        const SCREENS: &str = r#"[
    		{ "title": "From address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "indent": 2 },
    		{ "title": "To address", "content": "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t", "indent": 2 },
            { "title": "Amount", "content": "10 ATOM, 10'000'000 ucosm", "indent": 2 }
    	]"#;

        let expected_screens: Vec<Screen> = serde_json::from_str(SCREENS)?;

        let actual_screens = ValueRenderer::format(&msg, &TestMetadataGetter);

        assert!(actual_screens.is_ok(), "Failed to retrieve screens");
        assert_eq!(expected_screens, actual_screens.expect("Unreachable"));

        Ok(())
    }

    #[test]
    fn msg_send_works() -> anyhow::Result<()> {
        const MESSAGE: &str = r#"{
            "from_address": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs",
            "to_address": "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t",
            "amount": [{ "denom": "uatom", "amount": "10000000" }]
        }"#;

        let msg: MsgSend = serde_json::from_str(MESSAGE)?;

        const SCREENS: &str = r#"[
    		{ "title": "From address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "indent": 2 },
    		{ "title": "To address", "content": "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t", "indent": 2 },
            { "title": "Amount", "content": "10 ATOM", "indent": 2 }
    	]"#;

        let expected_screens: Vec<Screen> = serde_json::from_str(SCREENS)?;

        let actual_screens = ValueRenderer::format(&msg, &TestMetadataGetter);

        assert!(actual_screens.is_ok(), "Failed to retrieve screens");
        assert_eq!(expected_screens, actual_screens.expect("Unreachable"));

        Ok(())
    }
}
