use std::collections::HashMap;
use std::fmt::Display;

use layout21::raw::geom::Dir;
use layout21::raw::{Cell, LayerKey, Rect};
use layout21::utils::Ptr;
use serde::{Deserialize, Serialize};

use crate::config::Int;
use crate::Ref;
use crate::{config::Uint, Pdk};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, derive_builder::Builder)]
pub struct ContactParams {
    pub stack: String,
    pub rows: Uint,
    pub cols: Uint,
    /// The "relaxed" direction, ie. the direction in which there is more margin (for overhangs,
    /// for instance).
    ///
    /// If the contact generator needs more space, it will try to expand in
    /// this direction first.
    pub dir: Dir,
}

#[derive(Debug, Clone, Eq, PartialEq, derive_builder::Builder)]
pub struct Contact {
    pub cell: Ptr<Cell>,
    pub rows: Uint,
    pub cols: Uint,
    pub bboxes: HashMap<LayerKey, Rect>,
}

impl Display for ContactParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}x{}{}",
            &self.stack,
            self.rows,
            self.cols,
            self.dir.short_form()
        )
    }
}

impl ContactParams {
    pub fn builder() -> ContactParamsBuilder {
        ContactParamsBuilder::default()
    }
}

impl Pdk {
    pub fn get_contact(&self, params: &ContactParams) -> Ref<Contact> {
        let mut map = self.contacts.write().unwrap();
        if let Some(c) = map.get(params) {
            c.clone()
        } else {
            let c = self.draw_contact(params);
            map.insert(params.to_owned(), c.clone());
            c
        }
    }

    pub fn get_contact_sized(
        &self,
        stack: impl Into<String>,
        layer: LayerKey,
        width: Int,
    ) -> Option<Ref<Contact>> {
        let mut low = 1;
        let mut high = 100;
        let mut result = None;

        let stack = stack.into();

        while high > low {
            let mid = (high + low) / 2;
            let params = ContactParams::builder()
                .rows(1)
                .cols(mid)
                .stack(stack.clone())
                .dir(Dir::Horiz)
                .build()
                .unwrap();
            let ct = self.get_contact(&params);
            let bbox = ct.bboxes.get(&layer).unwrap();

            if bbox.p1.x - bbox.p0.x <= width {
                result = Some(ct);
                low = mid + 1;
            } else {
                high = mid;
            }
        }

        result
    }

    /// Gets the largest contact whose boundary on `layer` fits within the provided [`Rect`]'s
    /// width and height.
    ///
    /// Contacts with more than 100x100 units are not supported.
    pub fn get_contact_within(
        &self,
        stack: impl Into<String>,
        layer: LayerKey,
        bbox: impl Into<Rect>,
    ) -> Option<Ref<Contact>> {
        let mut low_r = 1;
        let mut high_r = 100;
        let mut low_c = 1;
        let mut high_c = 100;

        let stack = stack.into();
        let bbox = bbox.into();
        let dir = if bbox.width() > bbox.height() {
            Dir::Horiz
        } else {
            Dir::Vert
        };

        let mut result;

        loop {
            if high_r < low_r || high_c < low_c {
                return None;
            }

            assert!(high_r >= low_r);
            assert!(high_c >= low_c);

            let r = (low_r + high_r + 1) / 2;
            let c = (low_c + high_c + 1) / 2;

            let params = ContactParams::builder()
                .rows(r)
                .cols(c)
                .stack(stack.clone())
                .dir(dir)
                .build()
                .unwrap();
            let ct = self.get_contact(&params);
            let outline = ct.bboxes.get(&layer).unwrap();

            match (
                outline.width() <= bbox.width(),
                outline.height() <= bbox.height(),
            ) {
                (true, true) => {
                    result = Some(ct);
                    low_r = r;
                    low_c = c;
                    if r == high_r && c == high_c {
                        break;
                    }
                }
                (true, false) => {
                    low_c = c;
                    high_r = r - 1;
                }
                (false, true) => {
                    low_r = r;
                    high_c = c - 1;
                }
                (false, false) => {
                    high_r = r - 1;
                    high_c = c - 1;
                }
            }
        }

        result
    }
}
