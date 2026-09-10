//! qga-math must match flux_hopf_lib fixtures. Not a third 24.

use qga_math::{hopf_map_classical, hurwitz_units, Q};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn hurwitz_24_match_fixture_exactly() {
    let raw = std::fs::read_to_string(fixture("hurwitz_units_v1.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let want = v["units"].as_array().unwrap();
    assert_eq!(want.len(), 24);
    let got = hurwitz_units();
    for (i, row) in want.iter().enumerate() {
        let nums: Vec<f64> = row
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(nums.len(), 4);
        let q = got[i];
        for (a, b) in [q.w(), q.x(), q.y(), q.z()].into_iter().zip(nums) {
            assert_eq!(a, b as f32, "unit {i} component mismatch");
        }
    }
}

#[test]
fn hopf_on_hurwitz_matches_fixture() {
    let raw = std::fs::read_to_string(fixture("hopf_hurwitz_v1.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let points = v["points"].as_array().unwrap();
    assert_eq!(points.len(), 24);
    let units = hurwitz_units();
    let ulp = f32::EPSILON as f64 * 8.0;
    for (i, row) in points.iter().enumerate() {
        let q_json: Vec<f64> = row["q"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let y_json: Vec<f64> = row["y"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let q = units[i];
        for (a, b) in [q.w(), q.x(), q.y(), q.z()].into_iter().zip(q_json) {
            assert_eq!(a, b as f32, "q[{i}]");
        }
        let y = hopf_map_classical(q);
        assert!(
            (y.x as f64 - y_json[0]).abs() <= ulp
                && (y.y as f64 - y_json[1]).abs() <= ulp
                && (y.z as f64 - y_json[2]).abs() <= ulp,
            "hopf[{i}] got {:?} want {:?}",
            [y.x, y.y, y.z],
            y_json
        );
    }
}

#[test]
fn classical_default_is_not_legacy_portal() {
    let q = Q::new(0.0, 0.0, 1.0, 0.0);
    let y = hopf_map_classical(q);
    assert!((y.x).abs() < 1e-6);
    assert!((y.y).abs() < 1e-6);
    assert!((y.z + 1.0).abs() < 1e-6);
}
