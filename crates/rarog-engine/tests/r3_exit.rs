use rarog_compositor::{FrameCause, FrameScheduler};
use rarog_engine::{RenderOptions, render_html};
use rarog_types::{Color, Size};

const R3_BACKLOG: &str = include_str!("../../../docs/R3-BACKLOG.md");

fn options() -> RenderOptions {
    RenderOptions {
        viewport: Size {
            width: 160.0,
            height: 80.0,
        },
        background: Color::WHITE,
    }
}

#[test]
fn r3_exit_manifest_has_no_open_items() {
    assert!(R3_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R3_BACKLOG
            .lines()
            .any(|line| line.trim_start().starts_with("- [ ]")),
        "R3 backlog contains an unchecked milestone item; move later work to ROADMAP.md or complete the Wings requirement"
    );
}

#[test]
fn r3_exit_fractional_grid_matches_equivalent_fixed_geometry() {
    let fractional = r#"<div style="display:grid;width:110px;height:20px;justify-content:flex-start;grid-template-columns:1fr 3fr;grid-template-rows:20px;column-gap:10px"><div style="background:#112233"></div><div style="background:#445566"></div></div>"#;
    let fixed = r#"<div style="display:grid;width:110px;height:20px;justify-content:flex-start;grid-template-columns:25px 75px;grid-template-rows:20px;column-gap:10px"><div style="background:#112233"></div><div style="background:#445566"></div></div>"#;

    let fractional =
        render_html(fractional, options()).expect("R3 fractional Grid fixture must render");
    let fixed = render_html(fixed, options()).expect("R3 fixed Grid fixture must render");

    assert_eq!(
        fractional.framebuffer.stable_hash64(),
        fixed.framebuffer.stable_hash64()
    );
}

#[test]
fn r3_exit_frame_scheduler_coalesces_causes_and_completes_identity() {
    let mut scheduler = FrameScheduler::new();
    scheduler.request(FrameCause::Scroll);
    scheduler.request(FrameCause::ResourceReady);

    let scheduled = scheduler
        .begin()
        .expect("R3 frame scheduler must begin")
        .expect("coalesced R3 frame request must exist");

    assert!(scheduled.reasons().contains(FrameCause::Scroll));
    assert!(scheduled.reasons().contains(FrameCause::ResourceReady));
    assert_eq!(scheduled.primary_cause(), FrameCause::ResourceReady);

    scheduler
        .complete(scheduled.id())
        .expect("R3 frame request identity must complete");
    assert!(scheduler.begin().unwrap().is_none());
}
