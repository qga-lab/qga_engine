//! Load `flux_hopf_lib.hopf.export_fiber_curves` JSON into engine fibers.
//! Do not sys.path Python; this is the schema the explorer already ships.

use anyhow::{bail, Context, Result};
use glam::Vec3;
use qga_math::{color_from_eta, hopf_coordinates, stereographic, Fiber, HopfConvention, Q};
use std::path::Path;

pub fn load_export_fiber_curves(path: &Path, convention: HopfConvention) -> Result<Vec<Fiber>> {
    let raw = std::fs::read_to_string(path).with_context(|| path.display().to_string())?;
    let v: serde_json::Value = serde_json::from_str(&raw).context("fiber JSON")?;
    let version = v.get("version").and_then(|x| x.as_u64()).unwrap_or(1);
    if version != 1 {
        bail!("export_fiber_curves version {version} not supported (want 1)");
    }
    let scale = v
        .pointer("/meta/scale")
        .and_then(|x| x.as_f64())
        .unwrap_or(2.0) as f32;
    let arr = v
        .get("fibers")
        .and_then(|x| x.as_array())
        .context("missing fibers[]")?;
    let mut out = Vec::with_capacity(arr.len());
    for (i, f) in arr.iter().enumerate() {
        let eta = f.get("eta").and_then(|x| x.as_f64()).unwrap_or(0.4) as f32;
        let xi1 = f.get("xi1").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
        let xyz = f
            .get("xyz")
            .and_then(|x| x.as_array())
            .with_context(|| format!("fiber {i} missing xyz"))?;
        let mut points = Vec::with_capacity(xyz.len());
        for p in xyz {
            let n = p.as_array().context("xyz row")?;
            anyhow::ensure!(n.len() >= 3, "xyz row needs 3");
            points.push(Vec3::new(
                n[0].as_f64().unwrap_or(0.0) as f32,
                n[1].as_f64().unwrap_or(0.0) as f32,
                n[2].as_f64().unwrap_or(0.0) as f32,
            ));
        }
        let mut s3 = Vec::new();
        if let Some(rows) = f.get("s3").and_then(|x| x.as_array()) {
            for q in rows {
                let n = q.as_array().context("s3 row")?;
                anyhow::ensure!(n.len() >= 4, "s3 row needs 4");
                s3.push(Q::new(
                    n[0].as_f64().unwrap_or(0.0) as f32,
                    n[1].as_f64().unwrap_or(0.0) as f32,
                    n[2].as_f64().unwrap_or(0.0) as f32,
                    n[3].as_f64().unwrap_or(0.0) as f32,
                ));
            }
        }
        if s3.len() != points.len() {
            s3.clear();
            let n = points.len().max(8);
            if points.len() != n {
                points.clear();
            }
            for k in 0..n {
                let xi2 = k as f32 * (std::f32::consts::TAU / n as f32);
                let q = hopf_coordinates(eta, xi1, xi2);
                s3.push(q);
                if points.len() < n {
                    points.push(stereographic(q, scale));
                }
            }
        }
        let base = if let Some(b) = f.get("base").and_then(|x| x.as_array()) {
            if b.len() >= 3 {
                Vec3::new(
                    b[0].as_f64().unwrap_or(0.0) as f32,
                    b[1].as_f64().unwrap_or(0.0) as f32,
                    b[2].as_f64().unwrap_or(0.0) as f32,
                )
            } else {
                qga_math::hopf_map(s3[0], convention)
            }
        } else {
            qga_math::hopf_map(s3[0], convention)
        };
        out.push(Fiber {
            eta,
            xi1,
            points,
            s3,
            base,
            color: color_from_eta(eta),
        });
    }
    anyhow::ensure!(!out.is_empty(), "fiber JSON had no fibers");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qga_math::HopfConvention;
    use std::io::Write;

    #[test]
    fn loads_version1_without_s3() {
        let dir = std::env::temp_dir();
        let path = dir.join("qga-fiber-fixture.json");
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            r#"{{"version":1,"meta":{{"scale":2.0}},"fibers":[{{"eta":0.6,"xi1":1.2,"xyz":[[0,0,0],[1,0,0],[0,1,0],[0,0,1],[1,1,0],[1,0,1],[0,1,1],[1,1,1]]}}]}}"#
        )
        .unwrap();
        let fibers = load_export_fiber_curves(&path, HopfConvention::Classical).unwrap();
        assert_eq!(fibers.len(), 1);
        assert_eq!(fibers[0].s3.len(), fibers[0].points.len());
        assert!(fibers[0].s3.len() >= 8);
    }
}
