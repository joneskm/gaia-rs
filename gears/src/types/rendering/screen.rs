use std::collections::BTreeMap;

use ciborium::{value::CanonicalValue, Value};
use nutype::nutype;
use serde::{Deserialize, Serialize};

const TITLE_KEY: u64 = 1;
const CONTENT_KEY: u64 = 2;
const INDENT_KEY: u64 = 3;
const EXPERT_KEY: u64 = 4;

/// Content is the text (sequence of Unicode code points) to display after
/// the Title, generally on the device's content section.
#[nutype(
    validate(not_empty),
    derive(Eq, PartialEq, AsRef, Debug, Clone, Deserialize, Serialize)
)]
pub struct Content(String);

/// Indent is the indentation level of the screen.
/// Zero indicates top-level.
#[nutype(
    validate(less_or_equal = 16),
    derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, Serialize)
)]
pub struct Indent(u8);

impl Indent {
    pub fn one() -> Indent {
        Indent::try_new(1).expect("indent is less than 16")
    }

    pub fn two() -> Indent {
        Indent::try_new(2).expect("indent is less than 16")
    }
}

/// Screen is the abstract unit of Textual rendering.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Screen {
    /// `title` is the text (sequence of Unicode code points) to display first,
    /// generally on the device's title section. It can be empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,

    /// `content` is the text (sequence of Unicode code points) to display after
    /// the `title`, generally on the device's content section. It must be
    /// ***non-empty***.
    pub content: Content,

    /// `indent` is the indentation level of the screen.
    /// Zero indicates top-level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indent: Option<Indent>,

    /// `expert` indicates that the screen should only be displayed
    /// via an opt-in from the user.
    #[serde(default, skip_serializing_if = "bool_skip")]
    pub expert: bool,
}

/// To make sure that we 1:1 compatible with Cosmos.SDK we skip field if it has `false` value
fn bool_skip(var: &bool) -> bool {
    !(*var)
}

impl Screen {
    pub fn cbor_map(&self) -> BTreeMap<CanonicalValue, ciborium::Value> {
        let mut map: BTreeMap<CanonicalValue, ciborium::Value> = BTreeMap::new();
        if !self.title.is_empty() {
            let _ = map.insert(
                Value::Integer(TITLE_KEY.into()).into(),
                Value::Text(self.title.clone()),
            );
            // ignore returned
        }

        let _ = map.insert(
            Value::Integer(CONTENT_KEY.into()).into(),
            ciborium::Value::Text(self.content.clone().into_inner()),
        ); // nutype made validation that content is not empty

        if let Some(indent) = self.indent {
            if indent.into_inner() > 0 {
                let _ = map.insert(
                    Value::Integer(INDENT_KEY.into()).into(),
                    Value::Integer(indent.into_inner().into()),
                );
            }
        }
        if self.expert {
            let _ = map.insert(
                Value::Integer(EXPERT_KEY.into()).into(),
                Value::Bool(self.expert),
            );
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ciborium::Value;
    use extensions::testing::UnwrapTesting;
    use serde_json::json;

    use crate::types::rendering::cbor::Cbor;

    use super::Screen;

    #[test]
    fn cbor_check_1() {
        let value = json!( [
                { "title": "Chain id", "content": "my-chain" },
                { "title": "Account number", "content": "1" },
                { "title": "Sequence", "content": "2" },
                { "title": "Address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "expert": true },
                { "title": "Public key", "content": "/cosmos.crypto.secp256k1.PubKey", "expert": true },
                { "title": "Key", "content": "02EB DD7F E4FD EB76 DC8A 205E F65D 790C D30E 8A37 5A5C 2528 EB3A 923A F1FB 4D79 4D", "indent": 1, "expert": true },
                { "content": "This transaction has 1 Message" },
                { "title": "Message (1/1)", "content": "/cosmos.bank.v1beta1.MsgSend", "indent": 1 },
                { "title": "From address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "indent": 2 },
                { "title": "To address", "content": "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t", "indent": 2 },
                { "title": "Amount", "content": "10 ATOM", "indent": 2 },
                { "content": "End of Message" },
                { "title": "Fees", "content": "0.002 ATOM" },
                { "title": "Gas limit", "content": "100'000", "expert": true },
                { "title": "Hash of raw bytes", "content": "785bd306ea8962cdb9600089bdd65f3dc029e1aea112dee69e19546c9adad86e", "expert": true }
        ] );

        let screens: Vec<Screen> = serde_json::from_value(value).expect("Invalid json");

        const CBOR : &str = "a1018fa20168436861696e20696402686d792d636861696ea2016e4163636f756e74206e756d626572026131a2016853657175656e6365026132a301674164647265737302782d636f736d6f7331756c6176336873656e7570737771666b77327933737570356b677471776e767161386579687304f5a3016a5075626c6963206b657902781f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657904f5a401634b657902785230324542204444374620453446442045423736204443384120323035452046363544203739304320443330452038413337203541354320323532382045423341203932334120463146422034443739203444030104f5a102781e54686973207472616e73616374696f6e206861732031204d657373616765a3016d4d6573736167652028312f312902781c2f636f736d6f732e62616e6b2e763162657461312e4d736753656e640301a3016c46726f6d206164647265737302782d636f736d6f7331756c6176336873656e7570737771666b77327933737570356b677471776e76716138657968730302a3016a546f206164647265737302782d636f736d6f7331656a726634637572327779366b667572673966326a707070326833616665356836706b6835740302a30166416d6f756e74026731302041544f4d0302a1026e456e64206f66204d657373616765a2016446656573026a302e3030322041544f4da30169476173206c696d697402673130302730303004f5a3017148617368206f66207261772062797465730278403738356264333036656138393632636462393630303038396264643635663364633032396531616561313132646565363965313935343663396164616438366504f5";

        let map = screens.iter().map(Screen::cbor_map).collect::<Vec<_>>();

        let mut final_map = BTreeMap::new();

        final_map.insert(Value::Integer(1.into()).into(), map);
        let mut bytes = Vec::new();

        final_map.encode(&mut bytes).expect("Failed to encode");

        validate_result([(bytes, CBOR)])
    }

    #[test]
    fn cbor_check_2() {
        let value = json!( [
            { "title": "Chain id", "content": "my-chain" },
			{ "title": "Account number", "content": "1" },
			{ "title": "Sequence", "content": "2" },
			{ "title": "Address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "expert": true },
			{ "title": "Public key", "content": "/cosmos.crypto.secp256k1.PubKey", "expert": true },
			{ "title": "Key", "content": "02EB DD7F E4FD EB76 DC8A 205E F65D 790C D30E 8A37 5A5C 2528 EB3A 923A F1FB 4D79 4D", "indent": 1, "expert": true },
			{ "content": "This transaction has 1 Message" },
			{ "title": "Message (1/1)", "content": "/A", "indent": 1 },
			{ "title": "BYTES", "content": "SHA-256=32BA 545C D070 3E09 0FFC D80F 20E7 1729 9D12 5D46 3728 8871 2B2D B2D7 CFD2 AA80", "indent": 2 },
			{ "content": "End of Message" },
			{ "title": "Fees", "content": "0.002 ATOM" },
			{ "title": "Gas limit", "content": "100'000", "expert": true },
			{ "title": "Hash of raw bytes", "content": "04241fbfa336b82b7fa9d3ad5d8706891798aa9a4978da9e0d994510d2664cd4", "expert": true }
        ] );

        let screens: Vec<Screen> = serde_json::from_value(value).unwrap_test();

        const CBOR : &str = "a1018da20168436861696e20696402686d792d636861696ea2016e4163636f756e74206e756d626572026131a2016853657175656e6365026132a301674164647265737302782d636f736d6f7331756c6176336873656e7570737771666b77327933737570356b677471776e767161386579687304f5a3016a5075626c6963206b657902781f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657904f5a401634b657902785230324542204444374620453446442045423736204443384120323035452046363544203739304320443330452038413337203541354320323532382045423341203932334120463146422034443739203444030104f5a102781e54686973207472616e73616374696f6e206861732031204d657373616765a3016d4d6573736167652028312f312902622f410301a3016542595445530278575348412d3235363d333242412035343543204430373020334530392030464643204438304620323045372031373239203944313220354434362033373238203838373120324232442042324437204346443220414138300302a1026e456e64206f66204d657373616765a2016446656573026a302e3030322041544f4da30169476173206c696d697402673130302730303004f5a3017148617368206f66207261772062797465730278403034323431666266613333366238326237666139643361643564383730363839313739386161396134393738646139653064393934353130643236363463643404f5";
        let map = screens.iter().map(Screen::cbor_map).collect::<Vec<_>>();

        let mut final_map = BTreeMap::new();

        final_map.insert(Value::Integer(1.into()).into(), map);
        let mut bytes = Vec::new();

        final_map.encode(&mut bytes).unwrap_test();

        validate_result([(bytes, CBOR)])
    }

    #[test]
    fn cbor_check_3() {
        let value = json!( [
            { "title": "Chain id", "content": "my-chain" },
			{ "title": "Account number", "content": "1" },
			{ "title": "Sequence", "content": "2" },
			{ "title": "Address", "content": "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs", "expert": true },
			{ "title": "Public key", "content": "/cosmos.crypto.secp256k1.PubKey", "expert": true },
			{ "title": "Key", "content": "02EB DD7F E4FD EB76 DC8A 205E F65D 790C D30E 8A37 5A5C 2528 EB3A 923A F1FB 4D79 4D", "indent": 1, "expert": true },
			{ "content": "This transaction has 1 Message" },
			{ "title": "Message (1/1)", "content": "/A", "indent": 1 },
			{ "title": "BYTES", "content": "D31D 76DF 5DB7", "indent": 2 },
			{ "content": "End of Message" },
			{ "title": "Fees", "content": "0.002 ATOM" },
			{ "title": "Gas limit", "content": "100'000", "expert": true },
			{ "title": "Hash of raw bytes", "content": "6dc9a7a96c0908380dc067f2066d43844b55f430ace369dc165cfa981061d8cf", "expert": true }
        ] );

        let screens: Vec<Screen> = serde_json::from_value(value).unwrap_test();

        const CBOR : &str = "a1018da20168436861696e20696402686d792d636861696ea2016e4163636f756e74206e756d626572026131a2016853657175656e6365026132a301674164647265737302782d636f736d6f7331756c6176336873656e7570737771666b77327933737570356b677471776e767161386579687304f5a3016a5075626c6963206b657902781f2f636f736d6f732e63727970746f2e736563703235366b312e5075624b657904f5a401634b657902785230324542204444374620453446442045423736204443384120323035452046363544203739304320443330452038413337203541354320323532382045423341203932334120463146422034443739203444030104f5a102781e54686973207472616e73616374696f6e206861732031204d657373616765a3016d4d6573736167652028312f312902622f410301a301654259544553026e44333144203736444620354442370302a1026e456e64206f66204d657373616765a2016446656573026a302e3030322041544f4da30169476173206c696d697402673130302730303004f5a3017148617368206f66207261772062797465730278403664633961376139366330393038333830646330363766323036366434333834346235356634333061636533363964633136356366613938313036316438636604f5";

        let map = screens.iter().map(Screen::cbor_map).collect::<Vec<_>>();

        let mut final_map = BTreeMap::new();

        final_map.insert(Value::Integer(1_i32.into()).into(), map);
        let mut bytes = Vec::new();

        final_map.encode(&mut bytes).unwrap_test();

        validate_result([(bytes, CBOR)])
    }

    fn validate_result<'a>(value: impl IntoIterator<Item = (Vec<u8>, &'a str)>) {
        for (i, expected) in value {
            let actual = data_encoding::HEXLOWER.encode(&i);
            assert_eq!(actual, expected.to_owned());
        }
    }
}
