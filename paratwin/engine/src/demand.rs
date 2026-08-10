//! Spatial demand field: Gaussian-mixture intensity over the metro grid,
//! implementing the marked spatial Poisson demand of Def. 2 of the report.

use crate::model::Scenario;

/// Three-cluster Gaussian mixture roughly shaped like a Jakarta-class metro:
/// CBD core, southern residential belt, eastern corridor.
pub struct DemandField {
    pub metro_km: f64,
    pub cells: usize,          // grid cells per side (1 km cells)
    pub weights: Vec<f64>,     // cell -> share of total demand (sums to 1)
}

impl DemandField {
    pub fn new(sc: &Scenario) -> Self {
        let cells = sc.metro_km.ceil() as usize;
        let m = sc.metro_km;
        let clusters = [
            // (cx, cy, sigma, weight)
            (0.50 * m, 0.55 * m, 0.16 * m, 0.45), // CBD
            (0.38 * m, 0.25 * m, 0.20 * m, 0.35), // south residential
            (0.78 * m, 0.60 * m, 0.14 * m, 0.20), // east corridor
        ];
        let mut weights = vec![0.0; cells * cells];
        let mut total = 0.0;
        for zy in 0..cells {
            for zx in 0..cells {
                let x = (zx as f64 + 0.5) * m / cells as f64;
                let y = (zy as f64 + 0.5) * m / cells as f64;
                let mut w = 1e-4; // faint uniform background
                for (cx, cy, s, cw) in clusters {
                    let d2 = ((x - cx).powi(2) + (y - cy).powi(2)) / (2.0 * s * s);
                    w += cw * (-d2).exp();
                }
                weights[zy * cells + zx] = w;
                total += w;
            }
        }
        for w in &mut weights {
            *w /= total;
        }
        DemandField { metro_km: sc.metro_km, cells, weights }
    }

    /// cumulative distribution for cell sampling
    pub fn cdf(&self) -> Vec<f64> {
        let mut acc = 0.0;
        self.weights
            .iter()
            .map(|w| {
                acc += w;
                acc
            })
            .collect()
    }

    pub fn cell_center(&self, idx: usize) -> (f64, f64) {
        let zx = idx % self.cells;
        let zy = idx / self.cells;
        let s = self.metro_km / self.cells as f64;
        ((zx as f64 + 0.5) * s, (zy as f64 + 0.5) * s)
    }
}
