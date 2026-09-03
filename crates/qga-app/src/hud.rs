//! Scene HUDs built from qga-gpu bitmap primitives.

use qga_gpu::{hud_quad, hud_stroke, hud_text, HudVert};
use qga_math::{E_INV2, R_RESIDUAL};
use qga_sim::OamHud;

const COL_PANEL: [f32; 4] = [0.02, 0.04, 0.08, 0.72];
const COL_AXIS: [f32; 4] = [0.75, 0.82, 0.92, 0.55];
const COL_S: [f32; 4] = [0.35, 0.85, 1.0, 0.95];
const COL_R: [f32; 4] = [1.0, 0.72, 0.28, 0.90];
const COL_E: [f32; 4] = [0.55, 1.0, 0.45, 0.90];
const COL_CORE: [f32; 4] = [1.0, 0.28, 0.58, 0.95];
const COL_ENV: [f32; 4] = [0.20, 0.62, 1.0, 0.95];
const COL_FLUX: [f32; 4] = [1.0, 0.40, 0.08, 0.95];
const COL_TEXT: [f32; 4] = [0.92, 0.95, 1.0, 0.92];

pub fn build_oam_hud(hud: &OamHud) -> Vec<HudVert> {
    let mut v = Vec::with_capacity(4096);
    let x0 = -0.96;
    let x1 = -0.42;
    let y0 = -0.94;
    let y1 = -0.50;
    hud_quad(&mut v, x0, y0, x1, y1, COL_PANEL);
    hud_stroke(&mut v, x0, y0, x1, y0, 0.004, COL_AXIS);
    hud_stroke(&mut v, x0, y0, x0, y1, 0.004, COL_AXIS);

    let map_x = |lt: f32| x0 + (lt / 2.0).clamp(0.0, 1.0) * (x1 - x0);
    let map_y = |s: f32| y0 + s.clamp(0.0, 1.05) * (y1 - y0);

    let y_r = map_y(R_RESIDUAL as f32);
    let y_e = map_y(E_INV2 as f32);
    hud_stroke(&mut v, x0, y_r, x1, y_r, 0.0035, COL_R);
    hud_stroke(&mut v, x0, y_e, x1, y_e, 0.0035, COL_E);

    if hud.curve.len() >= 2 {
        for w in hud.curve.windows(2) {
            let (x_a, y_a) = (map_x(w[0].0), map_y(w[0].1));
            let (x_b, y_b) = (map_x(w[1].0), map_y(w[1].1));
            hud_stroke(&mut v, x_a, y_a, x_b, y_b, 0.006, COL_S);
        }
    }
    let cx = map_x(hud.lambda_t);
    let cy = map_y(hud.survival);
    hud_quad(&mut v, cx - 0.008, cy - 0.012, cx + 0.008, cy + 0.012, COL_S);

    hud_text(&mut v, x0 + 0.02, y1 - 0.04, 0.011, "S vs lt", COL_TEXT);
    hud_text(&mut v, x1 - 0.16, y_r + 0.016, 0.009, "R", COL_R);
    hud_text(&mut v, x1 - 0.16, y_e - 0.028, 0.009, "e-2", COL_E);

    let phase = match hud.phase.name() {
        "pump" => "PUMP  envelope spreads",
        "relax" => "RELAX  core residual",
        _ => "HOLD  S near R / e-2",
    };
    hud_text(&mut v, x0 + 0.02, y0 + 0.025, 0.009, phase, COL_TEXT);

    let lx = -0.96;
    let ly = 0.90;
    hud_quad(&mut v, lx, ly - 0.16, lx + 0.46, ly + 0.06, COL_PANEL);
    chip(&mut v, lx + 0.02, ly + 0.01, COL_CORE);
    hud_text(&mut v, lx + 0.07, ly - 0.005, 0.011, "CORE persists", COL_CORE);
    chip(&mut v, lx + 0.02, ly - 0.04, COL_ENV);
    hud_text(
        &mut v,
        lx + 0.07,
        ly - 0.055,
        0.011,
        "ENVELOPE diffracts",
        COL_ENV,
    );
    chip(&mut v, lx + 0.02, ly - 0.09, COL_FLUX);
    hud_text(
        &mut v,
        lx + 0.07,
        ly - 0.105,
        0.011,
        "OAM FLUX to lattice",
        COL_FLUX,
    );
    hud_text(
        &mut v,
        -0.96,
        0.97,
        0.012,
        "arXiv:2607.16520  photonic analog of twisted fermion cores",
        COL_TEXT,
    );
    v
}

pub fn build_reveal_hud(panel: qga_sim::RevealPanel) -> Vec<HudVert> {
    use qga_sim::RevealPanel;
    let mut v = Vec::with_capacity(4000);
    hud_text(
        &mut v,
        -0.96,
        0.96,
        0.011,
        "LORENZ VISUALIZER NOT A ROAD  KEYS 7 8 9 D",
        COL_TEXT,
    );
    hud_quad(&mut v, -0.98, 0.68, 0.12, 0.92, COL_PANEL);
    match panel {
        RevealPanel::A => {
            hud_text(&mut v, -0.96, 0.86, 0.014, "A ROTOR NOT HEADER", COL_S);
            hud_text(&mut v, -0.96, 0.78, 0.010, "C+ CYAN  C- ORANGE", COL_TEXT);
            hud_text(&mut v, -0.96, 0.72, 0.010, "EXTRA ROTOR IS A KICK", COL_R);
        }
        RevealPanel::B => {
            hud_text(&mut v, -0.96, 0.86, 0.014, "B PAINT NOT LOCK", COL_E);
            hud_text(&mut v, -0.96, 0.78, 0.010, "C+ CYAN  C- ORANGE", COL_TEXT);
            hud_text(&mut v, -0.96, 0.72, 0.010, "CHAIN IS NOT NORTH", COL_R);
        }
        RevealPanel::C => {
            hud_text(&mut v, -0.96, 0.86, 0.014, "C PACK NO ROAD", COL_R);
            hud_text(&mut v, -0.96, 0.78, 0.010, "GOLD DOT IS PELOTON", COL_TEXT);
            hud_text(&mut v, -0.96, 0.72, 0.010, "LEFTOVER IS THE PACK", COL_TEXT);
        }
        RevealPanel::D => {
            hud_text(&mut v, -0.96, 0.86, 0.014, "D IDLE NO MIRROR", COL_CORE);
            hud_text(&mut v, -0.96, 0.78, 0.010, "NO EXTERNAL HEADING", COL_TEXT);
            hud_text(&mut v, -0.96, 0.72, 0.010, "OBTAINED STAYS FALSE", COL_TEXT);
        }
    }
    v
}

pub const PRESET_IDS: &[&str] = &[
    "cluster", "brown", "blue", "hazel", "amber", "green", "gray", "chromia", "dark",
];
pub const PRESET_LABELS: &[&str] = &[
    "CLUSTER", "BROWN", "BLUE", "HAZEL", "AMBER", "GREEN", "GREY", "CHROMIA", "DARK",
];

pub const VIEW_IDS: &[&str] = &["default", "grid"];
pub const VIEW_LABELS: &[&str] = &["DEFAULT", "3X3 GRID"];

pub fn palette_for_preset(id: &str) -> Option<u32> {
    match id {
        "cluster" => Some(0),
        "brown" => Some(1),
        "blue" => Some(2),
        "hazel" => Some(3),
        "amber" => Some(4),
        "green" => Some(5),
        "gray" | "grey" => Some(6),
        "chromia" | "heterochromia" => Some(7),
        "dark" => Some(8),
        _ => None,
    }
}

const PRE_X0: f32 = -0.98;
const PRE_X1: f32 = -0.68;
const VIEW_X0: f32 = -0.66;
const VIEW_X1: f32 = -0.36;
const PRE_TAB_Y1: f32 = 0.97;
const PRE_TAB_Y0: f32 = 0.88;
const PRE_ROW: f32 = 0.052;

fn text_width(msg: &str, s: f32) -> f32 {
    msg.chars().count() as f32 * s * 1.15
}

fn text_in_cell(
    v: &mut Vec<HudVert>,
    x0: f32,
    x1: f32,
    y0: f32,
    y1: f32,
    s: f32,
    msg: &str,
    c: [f32; 4],
) {
    let tw = text_width(msg, s);
    let th = 7.0 * s * 0.26;
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    hud_text(v, cx - tw * 0.5, cy + th * 0.5, s, msg, c);
}

fn draw_dropdown(
    v: &mut Vec<HudVert>,
    x0: f32,
    x1: f32,
    open: bool,
    selected: usize,
    title: &str,
    labels: &[&str],
) {
    hud_quad(v, x0, PRE_TAB_Y0, x1, PRE_TAB_Y1, COL_PANEL);
    text_in_cell(v, x0, x1, PRE_TAB_Y0, PRE_TAB_Y1, 0.011, title, COL_TEXT);
    if !open {
        return;
    }
    let n = labels.len();
    let y_bot = PRE_TAB_Y0 - n as f32 * PRE_ROW;
    hud_quad(v, x0, y_bot, x1, PRE_TAB_Y0, COL_PANEL);
    let inset = 0.012;
    for (i, label) in labels.iter().enumerate() {
        let y1 = PRE_TAB_Y0 - i as f32 * PRE_ROW;
        let y0 = y1 - PRE_ROW;
        if i == selected {
            let pad_y = (PRE_ROW - 0.034) * 0.5;
            hud_quad(
                v,
                x0 + inset,
                y0 + pad_y,
                x1 - inset,
                y1 - pad_y,
                [0.16, 0.32, 0.52, 0.92],
            );
        }
        text_in_cell(v, x0, x1, y0, y1, 0.010, label, COL_TEXT);
    }
}

pub fn build_cosmos_hud(
    preset_open: bool,
    preset_ix: usize,
    view_open: bool,
    view_ix: usize,
    grid: bool,
    tabs_hidden: bool,
) -> Vec<HudVert> {
    let mut v = Vec::with_capacity(4096);
    if !tabs_hidden {
        draw_dropdown(
            &mut v,
            PRE_X0,
            PRE_X1,
            preset_open,
            preset_ix,
            "PRESETS",
            PRESET_LABELS,
        );
        draw_dropdown(
            &mut v,
            VIEW_X0,
            VIEW_X1,
            view_open,
            view_ix,
            "VIEW",
            VIEW_LABELS,
        );
        if grid {
            let dim = [0.82, 0.88, 0.96, 0.78];
            for (i, label) in PRESET_LABELS.iter().enumerate() {
                let col = (i % 3) as f32;
                let row = (i / 3) as f32;
                let x0 = -1.0 + col * (2.0 / 3.0);
                let x1 = x0 + 2.0 / 3.0;
                let y0 = 1.0 - (row + 1.0) * (2.0 / 3.0);
                text_in_cell(&mut v, x0, x1, y0 + 0.012, y0 + 0.055, 0.010, label, dim);
            }
        }
    }
    v
}

fn menu_hit(ndc_x: f32, ndc_y: f32, open: bool, x0: f32, x1: f32, n: i32) -> Option<i32> {
    if ndc_x < x0 || ndc_x > x1 {
        return None;
    }
    if ndc_y >= PRE_TAB_Y0 && ndc_y <= PRE_TAB_Y1 {
        return Some(0);
    }
    if !open {
        return None;
    }
    let y_bot = PRE_TAB_Y0 - n as f32 * PRE_ROW;
    if ndc_y < y_bot || ndc_y > PRE_TAB_Y0 {
        return None;
    }
    let i = ((PRE_TAB_Y0 - ndc_y) / PRE_ROW).floor() as i32;
    if i >= 0 && i < n {
        Some(i + 1)
    } else {
        None
    }
}

pub fn preset_hit(ndc_x: f32, ndc_y: f32, open: bool) -> Option<i32> {
    menu_hit(
        ndc_x,
        ndc_y,
        open,
        PRE_X0,
        PRE_X1,
        PRESET_LABELS.len() as i32,
    )
}

pub fn view_hit(ndc_x: f32, ndc_y: f32, open: bool) -> Option<i32> {
    menu_hit(
        ndc_x,
        ndc_y,
        open,
        VIEW_X0,
        VIEW_X1,
        VIEW_LABELS.len() as i32,
    )
}

fn chip(v: &mut Vec<HudVert>, x: f32, y: f32, c: [f32; 4]) {
    hud_quad(v, x, y, x + 0.035, y + 0.028, c);
}
