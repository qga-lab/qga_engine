//! qga-math must match flux_hopf_lib fixtures. Not a third 24.
//!
//! Classical Hopf is the fixture SoT. Kingdom is `legacy_portal_map` and
//! must not be allowed to satisfy `hopf_hurwitz_v1`.

use qga_math::{hopf_map_classical, hopf_map_kingdom, hurwitz_units, Q};
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
fn hopf_hurwitz_v1_matches_fixture() {
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

#[test]
fn kingdom_must_not_satisfy_hopf_hurwitz_v1() {
    let raw = std::fs::read_to_string(fixture("hopf_hurwitz_v1.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let points = v["points"].as_array().unwrap();
    let units = hurwitz_units();
    let ulp = f32::EPSILON as f64 * 8.0;
    let mut mismatches = 0usize;
    for (i, row) in points.iter().enumerate() {
        let y_json: Vec<f64> = row["y"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        let y = hopf_map_kingdom(units[i]);
        if (y.x as f64 - y_json[0]).abs() > ulp
            || (y.y as f64 - y_json[1]).abs() > ulp
            || (y.z as f64 - y_json[2]).abs() > ulp
        {
            mismatches += 1;
        }
    }
    assert!(
        mismatches > 0,
        "Kingdom/legacy_portal_map must not satisfy hopf_hurwitz_v1"
    );
}

/// Engine replay of the L0 **census**, not of Model 2.
/// Golden `along`/`inter` wait for a `structure_group_adjacency` port.
/// Do not invent Delaunay or occupancy edges here.
#[test]
fn op1_l0_census_hurwitz_hopf_replay() {
    let raw = std::fs::read_to_string(fixture("op1_l0_structure_group_counts_v1.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["rule"].as_str().unwrap(), "structure_group");
    assert_eq!(v["set"].as_str().unwrap(), "L0");
    assert_eq!(v["n_points"].as_u64().unwrap(), 24);
    assert_eq!(v["distinct_bases"].as_u64().unwrap(), 6);
    assert_eq!(v["bases_are_octahedron_poles"].as_bool().unwrap(), true);
    let inter_ds = v["inter_ds"].as_f64().unwrap();
    assert!(
        (inter_ds - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
        "inter_ds {inter_ds} is not π/2"
    );
    // Census integers still in the JSON. Do not check them without a port:
    // along = 24, inter = 12.

    let poles = [
        [1.0f32, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    let units = hurwitz_units();
    assert_eq!(units.len(), 24);

    let mut counts = [0usize; 6];
    let mut scatter = 0.0f32;
    for q in units {
        let y = hopf_map_classical(q);
        let mut best = 0usize;
        let mut best_d = f32::MAX;
        for (i, p) in poles.iter().enumerate() {
            let dx = y.x - p[0];
            let dy = y.y - p[1];
            let dz = y.z - p[2];
            let d = (dx * dx + dy * dy + dz * dz).sqrt();
            if d < best_d {
                best_d = d;
                best = i;
            }
        }
        counts[best] += 1;
        if best_d > scatter {
            scatter = best_d;
        }
    }
    assert!(
        counts.iter().all(|&c| c == 4),
        "want 4 units per octahedron pole, got {counts:?}"
    );
    assert!(
        scatter < 1e-8,
        "Hopf scatter on each L0 fiber {scatter} is not < 1e-8"
    );

    // Adjacent octahedron poles (dot ≈ 0) are at d_S = π/2. Antipodes at π.
    // Theorem image of h(Λ0), not a Delaunay construction.
    let mut n_half_pi = 0usize;
    let mut n_pi = 0usize;
    for i in 0..6 {
        for j in (i + 1)..6 {
            let dot = (poles[i][0] * poles[j][0]
                + poles[i][1] * poles[j][1]
                + poles[i][2] * poles[j][2])
                .clamp(-1.0, 1.0);
            let d = (dot as f64).acos();
            if (d - std::f64::consts::FRAC_PI_2).abs() < 1e-5 {
                n_half_pi += 1;
            } else if (d - std::f64::consts::PI).abs() < 1e-5 {
                n_pi += 1;
            }
        }
    }
    assert_eq!(n_half_pi, 12, "octahedron 1-skeleton has 12 edges at π/2");
    assert_eq!(n_pi, 3, "octahedron has 3 antipodal pairs");
}

/// Occupancy / Delaunay counts. Ignored until qga-math has a thin Model 2 port.
/// Do not invent edges to make these green.
#[test]
#[ignore = "no structure_group_adjacency in qga-math"]
fn op1_l0_along_inter_counts_wait_for_port() {
    let raw = std::fs::read_to_string(fixture("op1_l0_structure_group_counts_v1.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(v["along"].as_u64().unwrap(), 24);
    assert_eq!(v["inter"].as_u64().unwrap(), 12);
}
