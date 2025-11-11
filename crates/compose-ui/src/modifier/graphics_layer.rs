use super::{inspector_metadata, GraphicsLayer, Modifier};
use crate::modifier_nodes::GraphicsLayerElement;

impl Modifier {
    pub fn graphics_layer(layer: GraphicsLayer) -> Self {
        let inspector_values = layer;
        Self::with_element(GraphicsLayerElement::new(layer)).with_inspector_metadata(
            inspector_metadata("graphicsLayer", move |info| {
                info.add_property("alpha", inspector_values.alpha.to_string());
                info.add_property("scale", inspector_values.scale.to_string());
                info.add_property("translationX", inspector_values.translation_x.to_string());
                info.add_property("translationY", inspector_values.translation_y.to_string());
            }),
        )
    }
}
