use super::{inspector_metadata, Modifier};
use crate::modifier_nodes::WeightElement;

impl Modifier {
    pub fn weight(weight: f32) -> Self {
        Self::weight_with_fill(weight, true)
    }

    pub fn weight_with_fill(weight: f32, fill: bool) -> Self {
        Self::with_element(WeightElement::new(weight, fill)).with_inspector_metadata(
            inspector_metadata("weight", move |info| {
                info.add_property("weight", weight.to_string());
                info.add_property("fill", fill.to_string());
            }),
        )
    }

    pub fn columnWeight(self, weight: f32, fill: bool) -> Self {
        self.then(Self::weight_with_fill(weight, fill))
    }

    pub fn rowWeight(self, weight: f32, fill: bool) -> Self {
        self.then(Self::weight_with_fill(weight, fill))
    }
}
