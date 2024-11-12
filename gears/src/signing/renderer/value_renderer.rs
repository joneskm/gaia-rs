//! Trait for formatting all kind of values into `Screen`

use crate::{
    signing::handler::MetadataGetter,
    types::rendering::screen::{Content, Screen},
};

/// Render primitive type into content for `Screen`.
pub trait PrimitiveValueRenderer<V> {
    /// Get string representation of some `V` wrapped in a Content
    fn format(value: V) -> Content;
}

pub trait TryPrimitiveValueRenderer<V> {
    /// Try to get a string representation of some `V` wrapped in a Content
    fn try_format(value: V) -> Result<Content, RenderError>;
}

pub trait TryPrimitiveValueRendererWithMetadata<V> {
    /// Try to get a string representation of some `V` wrapped in a Content. This method also
    /// takes a function to get metadata for the denom.
    fn try_format_with_metadata<MG: MetadataGetter>(
        value: V,
        get_metadata: &MG,
    ) -> Result<Content, RenderError>;
}

#[cfg_attr(test, derive(PartialEq, Eq))]
#[derive(thiserror::Error, Debug)]
pub enum RenderError {
    #[error("not implemented")]
    NotImplemented,
    #[error("`{0}`")]
    Rendering(String),
}

pub trait ValueRenderer {
    /// Format renders the Protobuf value to a list of Screens.
    fn format<MG: MetadataGetter>(&self, get_metadata: &MG) -> Result<Vec<Screen>, RenderError>;
}

/// Default implementation of `PrimitiveValueRenderer` for `Screen`. This is an attempt
/// at a blanket implementation for all primitive types described in the Cosmos SDK:
/// https://docs.cosmos.network/v0.50/build/architecture/adr-050-sign-mode-textual-annex1#bytes
#[derive(Debug)]
pub struct DefaultPrimitiveRenderer;
