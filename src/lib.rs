use std::{collections::HashMap, path::Path};

use arcstr::ArcStr;
use config::TechConfig;
use contact::{Contact, ContactParams};
use layout21::gds21::GdsError;
use layout21::raw::{LayoutError, Units};
use layout21::{
    gds21::GdsLibrary,
    raw::{Cell, DepOrder, LayerKey, Layers, LayoutResult, Library},
    utils::{Ptr, PtrList},
};
use mos::{LayoutTransistors, MosParams, MosResult};

use crate::config::Int;

pub type Ref<T> = std::sync::Arc<T>;
pub type LayerIdx = u32;

pub mod bus;
pub mod config;
pub mod contact;
pub mod gds;
pub mod geometry;
pub mod mos;
pub mod tech;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pdk {
    pub tech: ArcStr,
    pub config: Ptr<TechConfig>,
    pub layers: Ptr<Layers>,
    contacts: Ptr<HashMap<ContactParams, Ref<Contact>>>,
}

#[derive(Debug, Clone)]
pub struct PdkLib {
    pub tech: ArcStr,
    pub pdk: Pdk,
    pub lib: Library,
    ptx: HashMap<MosParams, Ref<LayoutTransistors>>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("layout error: {0}")]
    Layout(#[from] LayoutError),
    #[error("GDS error: {0}")]
    Gds(#[from] GdsError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl PdkLib {
    /// Exports this library to GDS structures.
    ///
    /// Does not save anything to disk.
    pub fn export_gds(&self) -> LayoutResult<GdsLibrary> {
        let mut lib = Library::new(self.lib.name.clone(), self.pdk.config.read().unwrap().units);
        lib.layers = self.pdk.layers();
        lib.cells = self.lib.cells.clone();
        lib.cells = PtrList::from_ptrs(DepOrder::order(&lib));

        lib.to_gds()
    }

    /// Exports this library to GDS and saves it at the given file path.
    ///
    /// Creates the parent directory if it does not already exist.
    pub fn save_gds(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let gds = self.export_gds()?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        gds.save(&path)?;

        Ok(())
    }

    pub fn cell(&self, name: &str) -> Option<Ptr<Cell>> {
        self.lib.cell(name)
    }

    pub fn draw_contact(&mut self, params: &ContactParams) -> Ref<Contact> {
        let ct = self.pdk.draw_contact(params);
        self.lib.cells.push(ct.cell.clone());
        ct
    }

    pub fn draw_mos(&mut self, params: MosParams) -> MosResult<Ref<LayoutTransistors>> {
        if let Some(ptx) = self.ptx.get(&params) {
            return Ok(ptx.clone());
        }

        let ptx = match &*self.tech {
            "sky130" => self.pdk.draw_sky130_mos(params.clone()),
            _ => panic!("unsupported technology: {}", &self.tech),
        }?;

        self.ptx.insert(params, ptx.clone());

        Ok(ptx)
    }
}

// Rounds a to the nearest multiple of b
#[inline]
fn round(a: Int, b: Int) -> Int {
    assert!(b > 0);
    let min = (a / b) * b;
    let max = min + b;
    if a - min < max - a {
        min
    } else {
        max
    }
}

impl Pdk {
    pub fn new(tech: ArcStr, config: TechConfig) -> LayoutResult<Self> {
        let layers = Ptr::new(config.get_layers()?);
        let config = Ptr::new(config);
        Ok(Self {
            tech,
            config,
            layers,
            contacts: Ptr::new(HashMap::new()),
        })
    }

    pub fn create_lib(&self, name: impl Into<String>) -> Library {
        Library {
            name: name.into(),
            units: self.units(),
            layers: self.layers(),
            cells: PtrList::new(),
        }
    }

    pub fn create_pdk_lib(&self, name: impl Into<String>) -> PdkLib {
        PdkLib {
            tech: self.tech.clone(),
            lib: self.create_lib(name),
            pdk: self.clone(),
            ptx: HashMap::new(),
        }
    }

    pub fn units(&self) -> Units {
        let tc = self.config.read().unwrap();
        tc.units
    }

    pub fn grid(&self) -> Int {
        let tc = self.config.read().unwrap();
        tc.grid
    }

    pub fn gridded_center_span(&self, center: Int, span: Int) -> (Int, Int) {
        let grid = self.grid();
        // Span must be a multiple of the grid size
        assert!(span % grid == 0);

        let xmin = round(center - span / 2, grid);
        let xmax = xmin + span;

        assert!(xmax - xmin == span);

        (xmin, xmax)
    }

    #[inline]
    pub fn config(&self) -> Ptr<TechConfig> {
        Ptr::clone(&self.config)
    }

    #[inline]
    pub fn layers(&self) -> Ptr<Layers> {
        Ptr::clone(&self.layers)
    }

    pub fn cell_to_gds(&self, cell: Ptr<Cell>, path: impl AsRef<Path>) -> Result<(), Error> {
        let cell_name = {
            let cell = cell.read().unwrap();
            cell.name.to_owned()
        };
        let mut lib = Library::new(&cell_name, self.config.read().unwrap().units);
        lib.layers = self.layers();
        lib.cells.push(cell);
        let gds = lib.to_gds()?;
        gds.save(path)?;
        Ok(())
    }

    pub fn get_layerkey(&self, layer: &str) -> Option<LayerKey> {
        let layers = self.layers.read().unwrap();
        layers.keyname(layer)
    }
}

// #[cfg(test)]
// mod tests {
//     use layout21::{raw::{Library, Cell}, utils::Ptr};
//
//     #[test]
//     fn test_draw_mos() -> Result<(), Box<dyn std::error::Error>> {
//         let tc = crate::sky130_config();
//         let layout = crate::draw_mos((), &tc)?;
//         let mut cell = Cell::new("ptx");
//         cell.layout = Some(layout);
//
//         let mut lib = Library::new("test_draw_mos", tc.units);
//         lib.layers = Ptr::new(tc.get_layers()?);
//         lib.cells.push(Ptr::new(cell));
//         let gds = lib.to_gds()?;
//         gds.save("hi.gds")?;
//
//         Ok(())
//     }
// }
