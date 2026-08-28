from pathlib import Path


def replace(path: str, old: str, new: str, count: int = 1) -> None:
    file = Path(path)
    text = file.read_text(encoding="utf-8")
    found = text.count(old)
    if found != count:
        raise SystemExit(f"{path}: expected {count}, found {found}: {old[:120]!r}")
    file.write_text(text.replace(old, new), encoding="utf-8")


layout = "crates/rarog-layout/src/lib.rs"
replace(
    layout,
    """            runs[1],
            BidiRun {
                range: TextRange::new(4, 9),
                level: BidiLevel::new(1)
            }
        );
        assert_eq!(
            runs[2],
            BidiRun {
                range: TextRange::new(9, 12),
                level: BidiLevel::new(0)
            }""",
    """            runs[1],
            BidiRun {
                range: TextRange::new(4, 8),
                level: BidiLevel::new(1)
            }
        );
        assert_eq!(
            runs[2],
            BidiRun {
                range: TextRange::new(8, 12),
                level: BidiLevel::new(0)
            }""",
)
replace(
    layout,
    """                ShapingRun {
                    range: TextRange::new(4, 9),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(1),
                },
                ShapingRun {
                    range: TextRange::new(9, 11),
                    face: FontFaceId::new(2),
                    level: BidiLevel::new(0),
                },""",
    """                ShapingRun {
                    range: TextRange::new(4, 8),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(1),
                },
                ShapingRun {
                    range: TextRange::new(8, 9),
                    face: FontFaceId::new(1),
                    level: BidiLevel::new(0),
                },
                ShapingRun {
                    range: TextRange::new(9, 11),
                    face: FontFaceId::new(2),
                    level: BidiLevel::new(0),
                },""",
)
replace(
    layout,
    """        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].direction(), TextDirection::Ltr);
        assert_eq!(segments[1].direction(), TextDirection::Rtl);
        assert_eq!(segments[2].direction(), TextDirection::Ltr);""",
    """        assert_eq!(segments.len(), 4);
        assert_eq!(segments[0].direction(), TextDirection::Ltr);
        assert_eq!(segments[1].direction(), TextDirection::Rtl);
        assert_eq!(segments[2].direction(), TextDirection::Ltr);
        assert_eq!(segments[3].direction(), TextDirection::Ltr);""",
)
replace(
    layout,
    """        assert_eq!(shaped.len(), 3);
        assert_eq!(shaped[0].run.face, FontFaceId::new(0));
        assert_eq!(shaped[1].run.face, FontFaceId::new(1));
        assert_eq!(shaped[2].run.face, FontFaceId::new(2));
        assert_eq!(shaped[1].run.direction(), TextDirection::Rtl);""",
    """        assert_eq!(shaped.len(), 4);
        assert_eq!(shaped[0].run.face, FontFaceId::new(0));
        assert_eq!(shaped[1].run.face, FontFaceId::new(1));
        assert_eq!(shaped[2].run.face, FontFaceId::new(1));
        assert_eq!(shaped[3].run.face, FontFaceId::new(2));
        assert_eq!(shaped[1].run.direction(), TextDirection::Rtl);
        assert_eq!(shaped[2].run.direction(), TextDirection::Ltr);""",
)
replace(
    layout,
    """fn is_breakable_whitespace(character: char) -> bool {
    character.is_whitespace()
        && !is_mandatory_break(character)
        && !matches!(character, '\\u{00a0}' | '\\u{202f}')
}

fn is_cjk_ideograph(character: char) -> bool {
    matches!(
        character as u32,
        0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f
    )
}

""",
    """,
)

engine = "crates/rarog-engine/src/lib.rs"
replace(
    engine,
    """        session.update();

        assert_eq!(session.document().mutation_record_count(), 0);""",
    """        session
            .update()
            .expect("metadata update succeeds");

        assert_eq!(session.document().mutation_record_count(), 0);""",
)
