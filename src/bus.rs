use crate::config::Int;
use crate::contact::ContactParams;
use crate::{LayerIdx, Pdk};

use serde::{Deserialize, Serialize};

/// Specifies how contacts should be placed on a bus.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct ContactPolicy {
    above: Option<ContactPosition>,
    below: Option<ContactPosition>,
}

/// Specifies how contacts should be placed on any given layer.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ContactPosition {
    /// Contacts are centered on the bus, and cannot be placed in the same.
    /// position on adjacent traces.
    CenteredNonAdjacent,
    /// Contacts are centered on the bus, but can be placed on adjacent traces.
    CenteredAdjacent,
}

impl Pdk {
    /// The minimum spacing between tracks on a bus, assuming minimum sized contacts is used.
    pub fn bus_min_spacing(&self, metal: LayerIdx, width: Int, strategy: ContactPolicy) -> Int {
        use std::cmp::{max, min};

        let tc = self.config.read().unwrap();
        let space = tc.layer(self.metal_name(metal)).space;
        let mut min_space = space;

        if let Some(above) = strategy.above {
            let params = ContactParams::builder()
                .stack(self.stack_name(metal).to_string())
                .rows(1)
                .cols(1)
                .dir(layout21::raw::Dir::Vert)
                .build()
                .expect("Failed to build contact params");
            let ct = self.get_contact(&params);
            let rect = ct.bboxes.get(&self.metal(metal)).unwrap();
            let ct_width = min(rect.height(), rect.width());

            match above {
                ContactPosition::CenteredAdjacent => {
                    let overhang = max(ct_width - width, 0);
                    min_space = max(min_space, space + overhang);
                }
                ContactPosition::CenteredNonAdjacent => {
                    // The plus 1 is to round up.
                    let overhang = max((ct_width - width + 1) / 2, 0);
                    min_space = max(min_space, space + overhang);
                }
            }
        }

        if let Some(below) = strategy.below {
            if metal == 0 {
                panic!("Cannot contact the lowest metal layer from below ");
            }
            let params = ContactParams::builder()
                .stack(self.stack_name(metal - 1).to_string())
                .rows(1)
                .cols(1)
                .dir(layout21::raw::Dir::Vert)
                .build()
                .expect("Failed to build contact params");
            let ct = self.get_contact(&params);
            let rect = ct.bboxes.get(&self.metal(metal)).unwrap();
            let ct_width = min(rect.height(), rect.width());

            match below {
                ContactPosition::CenteredAdjacent => {
                    let overhang = max(ct_width - width, 0);
                    min_space = max(min_space, space + overhang);
                }
                ContactPosition::CenteredNonAdjacent => {
                    // The plus 1 is to round up.
                    let overhang = max((ct_width - width + 1) / 2, 0);
                    min_space = max(min_space, space + overhang);
                }
            }
        }

        min_space
    }
}
