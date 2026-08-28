use rarog_engine::{RenderOptions, render_html};
use rarog_types::{Color, Size};

const R0_BACKLOG: &str = include_str!("../../../docs/R0-BACKLOG.md");
const FIXTURE: &str = "<style>.card { width:80px; padding:4px; background:#112233; } #hero { border-width:2px; border-color:#000000; }</style><div id=\"hero\" class=\"card\">Rarog</div>";

#[test]
fn r0_exit_manifest_has_no_open_items() {
    assert!(R0_BACKLOG.contains("Status: **complete**."));
    assert!(
        !R0_BACKLOG.lines().any(|line| line.trim_start().starts_with("- [ ]")),
        "R0 backlog contains an unchecked milestone item; move later work to ROADMAP.md or complete the Ember requirement"
    );
}

#[test]
fn r0_exit_render_contract_is_deterministic() {
    let options = RenderOptions {
        viewport: Size {
            width: 160.0,
            height: 90.0,
        },
        background: Color::WHITE,
    };

    let first = render_html(FIXTURE, options).expect("R0 fixture must render");
    let second = render_html(FIXTURE, options).expect("R0 fixture must render repeatedly");

    assert_eq!(first.document.snapshot(), second.document.snapshot());
    assert_eq!(first.styles.snapshot(), second.styles.snapshot());
    assert_eq!(
        first.layout.tree.style_snapshot(),
        second.layout.tree.style_snapshot()
    );
    assert_eq!(first.layout.tree.snapshot(), second.layout.tree.snapshot());
    assert_eq!(
        first.layout.fragments.snapshot(),
        second.layout.fragments.snapshot()
    );
    assert_eq!(first.display_list.snapshot(), second.display_list.snapshot());
    assert_eq!(
        first.framebuffer.stable_hash64(),
        second.framebuffer.stable_hash64()
    );
    assert_eq!(
        first.deterministic_signature_hash(),
        second.deterministic_signature_hash()
    );

    assert!(first.observability.counters.dom_nodes > 0);
    assert!(first.observability.counters.layout_nodes > 0);
    assert!(first.observability.counters.fragments > 0);
    assert!(first.observability.counters.display_commands > 0);
}
