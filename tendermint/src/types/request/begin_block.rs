use crate::types::proto::{
    header::Header,
    info::{Evidence, LastCommitInfo},
};

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Debug)]
pub struct RequestBeginBlock {
    pub hash: ::prost::bytes::Bytes,
    pub header: Header,
    pub last_commit_info: LastCommitInfo,
    pub byzantine_validators: Vec<Evidence>,
}

impl From<RequestBeginBlock> for super::inner::RequestBeginBlock {
    fn from(
        RequestBeginBlock {
            hash,
            header,
            last_commit_info,
            byzantine_validators,
        }: RequestBeginBlock,
    ) -> Self {
        Self {
            hash,
            header: Some(header.into()),
            last_commit_info: Some(last_commit_info.into()),
            byzantine_validators: byzantine_validators.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<super::inner::RequestBeginBlock> for RequestBeginBlock {
    type Error = crate::error::Error;

    fn try_from(
        super::inner::RequestBeginBlock {
            hash,
            header,
            last_commit_info,
            byzantine_validators,
        }: super::inner::RequestBeginBlock,
    ) -> Result<Self, Self::Error> {
        let mut byzantine_validators_res = vec![];
        for byz_validator in byzantine_validators {
            byzantine_validators_res.push(byz_validator.try_into()?);
        }
        Ok(Self {
            hash,
            header: header
                .ok_or_else(|| crate::error::Error::InvalidData("header is missing".into()))?
                .try_into()?,
            last_commit_info: last_commit_info
                .ok_or_else(|| {
                    crate::error::Error::InvalidData("last_commit_info is missing".into())
                })?
                .try_into()?,
            byzantine_validators: byzantine_validators_res,
        })
    }
}
